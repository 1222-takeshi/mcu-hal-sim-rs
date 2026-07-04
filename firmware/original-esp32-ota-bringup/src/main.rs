//! original-esp32-ota-bringup
//!
//! Connects to WiFi then listens on HTTP port 8080 for OTA firmware uploads.
//! A firmware image POSTed to `POST /ota` is written to the inactive OTA slot;
//! on success the device replies with `200 OK` and reboots into the new image.

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::reset;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::ota::EspOta;
use esp_idf_svc::sys::{self, EspError};
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};

const WIFI_SSID: &str = env!("OTA_WIFI_SSID");
const WIFI_PSK: &str = env!("OTA_WIFI_PSK");
const OTA_AUTH_TOKEN: &str = env!("OTA_AUTH_TOKEN");

const OTA_TCP_PORT: u16 = 8080;
const MAX_OTA_SIZE: usize = 0x1E0000;
const OTA_CHUNK_SIZE: usize = 512;
const MAX_HEADER_LINE_LEN: usize = 512;
const MAX_HEADER_BYTES: usize = 2048;
const READ_TIMEOUT: Duration = Duration::from_secs(120);
const REBOOT_DELAY: Duration = Duration::from_millis(500);

type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
enum AppError {
    Esp(EspError),
    Io(io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Esp(err) => write!(f, "ESP-IDF error: {err}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<EspError> for AppError {
    fn from(value: EspError) -> Self {
        Self::Esp(value)
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
enum OtaRequestError {
    BadRequest,
    Unauthorized,
    HeaderTooLarge,
    LengthRequired,
    PayloadTooLarge(usize),
    Io(io::Error),
}

impl From<io::Error> for OtaRequestError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl fmt::Display for OtaRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest => write!(f, "bad request"),
            Self::Unauthorized => write!(f, "missing or invalid OTA token"),
            Self::HeaderTooLarge => write!(f, "request header too large"),
            Self::LengthRequired => write!(f, "missing or invalid Content-Length"),
            Self::PayloadTooLarge(len) => write!(f, "payload too large: {len} bytes"),
            Self::Io(err) => write!(f, "I/O error while reading request: {err}"),
        }
    }
}

#[derive(Debug)]
enum OtaWriteError {
    Esp(EspError),
    Io(io::Error),
    UnexpectedEof { expected: usize, received: usize },
}

impl From<EspError> for OtaWriteError {
    fn from(value: EspError) -> Self {
        Self::Esp(value)
    }
}

impl From<io::Error> for OtaWriteError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl fmt::Display for OtaWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Esp(err) => write!(f, "ESP-IDF OTA error: {err}"),
            Self::Io(err) => write!(f, "I/O error while receiving image: {err}"),
            Self::UnexpectedEof { expected, received } => {
                write!(
                    f,
                    "upload ended early: received {received} / {expected} bytes"
                )
            }
        }
    }
}

fn main() -> AppResult<()> {
    sys::link_patches();
    EspLogger::initialize_default();

    println!("[ota] booting ESP-IDF OTA receiver");

    let mut ota = EspOta::new()?;
    match ota.mark_running_slot_valid() {
        Ok(()) => println!("[ota] current slot marked as valid"),
        Err(err) => println!("[ota] current slot validation skipped: {err:?}"),
    }

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
        sysloop,
    )?;

    connect_wifi(&mut wifi)?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    println!(
        "[ota] IP address: {}  OTA port: {}",
        ip_info.ip, OTA_TCP_PORT
    );
    println!("[ota] ready - waiting for firmware upload");
    println!(
        "[ota] upload with:  ./scripts/flash-esp32.sh <firmware> --ota {}",
        ip_info.ip
    );

    let listener = TcpListener::bind(("0.0.0.0", OTA_TCP_PORT))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if handle_client(stream, &mut ota) {
                    println!("[ota] rebooting into updated firmware");
                    thread::sleep(REBOOT_DELAY);
                    reset::restart();
                }
            }
            Err(err) => println!("[ota] accept error: {err:?}"),
        }
    }

    Ok(())
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> AppResult<()> {
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().expect("OTA_WIFI_SSID is too long"),
        password: WIFI_PSK.try_into().expect("OTA_WIFI_PSK is too long"),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    println!("[ota] WiFi started, connecting to \"{}\"", WIFI_SSID);

    wifi.connect()?;
    wifi.wait_netif_up()?;
    println!("[ota] connected");

    Ok(())
}

fn handle_client(stream: TcpStream, ota: &mut EspOta) -> bool {
    if let Err(err) = stream.set_read_timeout(Some(READ_TIMEOUT)) {
        println!("[ota] failed to set read timeout: {err:?}");
    }
    if let Err(err) = stream.set_write_timeout(Some(Duration::from_secs(5))) {
        println!("[ota] failed to set write timeout: {err:?}");
    }

    let mut reader = BufReader::new(stream);
    let content_len = match read_ota_request(&mut reader) {
        Ok(len) => len,
        Err(err) => {
            println!("[ota] request rejected: {err}");
            let _ = write_error_response(reader.get_mut(), &err);
            return false;
        }
    };

    println!("[ota] receiving {} bytes of firmware", content_len);
    match write_ota_image(ota, &mut reader, content_len) {
        Ok(()) => {
            println!("[ota] write OK - flushing response then rebooting");
            let stream = reader.get_mut();
            let _ = stream.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 7\r\n\r\nOTA OK\n");
            let _ = stream.flush();
            true
        }
        Err(err) => {
            println!("[ota] write FAILED: {err}");
            let stream = reader.get_mut();
            let _ = stream.write_all(
                b"HTTP/1.0 500 Internal Server Error\r\nContent-Length: 11\r\n\r\nOTA FAILED\n",
            );
            let _ = stream.flush();
            false
        }
    }
}

fn read_ota_request<R: BufRead>(reader: &mut R) -> Result<usize, OtaRequestError> {
    let request_line = read_header_line(reader)?;
    if request_line.is_empty() {
        return Err(OtaRequestError::BadRequest);
    }
    let mut header_bytes = request_line.len();

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    if method != "POST" || path != "/ota" || !version.starts_with("HTTP/1.") {
        return Err(OtaRequestError::BadRequest);
    }

    let mut content_length = None;
    let mut ota_token_ok = false;
    loop {
        let line = read_header_line(reader)?;
        if line.is_empty() {
            return Err(OtaRequestError::BadRequest);
        }
        header_bytes += line.len();
        if header_bytes > MAX_HEADER_BYTES {
            return Err(OtaRequestError::HeaderTooLarge);
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        if let Some((name, value)) = trimmed.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse::<usize>().ok();
            } else if name.eq_ignore_ascii_case("x-ota-token") {
                ota_token_ok = value.trim() == OTA_AUTH_TOKEN;
            }
        }
    }

    if !ota_token_ok {
        return Err(OtaRequestError::Unauthorized);
    }

    let len = content_length.ok_or(OtaRequestError::LengthRequired)?;
    if len == 0 || len > MAX_OTA_SIZE {
        return Err(OtaRequestError::PayloadTooLarge(len));
    }

    Ok(len)
}

fn read_header_line<R: BufRead>(reader: &mut R) -> Result<String, OtaRequestError> {
    let mut line = Vec::new();

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let take_len = newline.map_or(available.len(), |pos| pos + 1);
        if line.len() + take_len > MAX_HEADER_LINE_LEN {
            return Err(OtaRequestError::HeaderTooLarge);
        }

        line.extend_from_slice(&available[..take_len]);
        reader.consume(take_len);

        if newline.is_some() {
            break;
        }
    }

    String::from_utf8(line).map_err(|_| OtaRequestError::BadRequest)
}

fn write_ota_image<R: Read>(
    ota: &mut EspOta,
    reader: &mut R,
    content_len: usize,
) -> Result<(), OtaWriteError> {
    let mut update = ota.initiate_update_with_known_size(content_len)?;
    let mut remaining = content_len;
    let mut received = 0;
    let mut buf = [0u8; OTA_CHUNK_SIZE];

    while remaining > 0 {
        let chunk_len = remaining.min(buf.len());
        let n = reader.read(&mut buf[..chunk_len])?;
        if n == 0 {
            return Err(OtaWriteError::UnexpectedEof {
                expected: content_len,
                received,
            });
        }

        update.write(&buf[..n])?;
        received += n;
        remaining -= n;

        if received % (64 * 1024) == 0 || received == content_len {
            println!("[ota] flashed {} / {} bytes", received, content_len);
        }
    }

    update.complete()?;
    Ok(())
}

fn write_error_response(stream: &mut TcpStream, err: &OtaRequestError) -> io::Result<()> {
    let response = match err {
        OtaRequestError::BadRequest => {
            b"HTTP/1.0 400 Bad Request\r\nContent-Length: 12\r\n\r\nBad Request\n".as_slice()
        }
        OtaRequestError::Unauthorized => {
            b"HTTP/1.0 401 Unauthorized\r\nContent-Length: 13\r\n\r\nUnauthorized\n".as_slice()
        }
        OtaRequestError::HeaderTooLarge => {
            b"HTTP/1.0 431 Request Header Fields Too Large\r\nContent-Length: 17\r\n\r\nHeader Too Large\n"
                .as_slice()
        }
        OtaRequestError::LengthRequired => {
            b"HTTP/1.0 411 Length Required\r\nContent-Length: 16\r\n\r\nLength Required\n"
                .as_slice()
        }
        OtaRequestError::PayloadTooLarge(_) => {
            b"HTTP/1.0 413 Payload Too Large\r\nContent-Length: 18\r\n\r\nPayload Too Large\n"
                .as_slice()
        }
        OtaRequestError::Io(_) => {
            b"HTTP/1.0 400 Bad Request\r\nContent-Length: 12\r\n\r\nBad Request\n".as_slice()
        }
    };
    stream.write_all(response)?;
    stream.flush()
}
