//! original-esp32-ota-bringup
//!
//! Connects to WiFi then listens on TCP port 8080 for OTA firmware uploads.
//! A new firmware binary POSTed to `POST /ota` is written to the inactive OTA
//! slot; on success the device reboots into the new image.
//!
//! Build and initial flash (USB serial, one-time):
//!
//! ```
//! OTA_WIFI_SSID=MyNet OTA_WIFI_PSK=MyPass cargo run --release
//! ```
//!
//! Subsequent updates over WiFi:
//!
//! ```
//! ./scripts/flash-esp32.sh firmware/original-esp32-ota-bringup --ota 192.168.1.42
//! ```
//!
//! OTA port:  8080  (plain TCP, minimal HTTP subset)
//! OTA IP:    printed to serial on startup

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::str;

use esp_backtrace as _;
use esp_bootloader_esp_idf::esp_app_desc;
use esp_hal::{
    main,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_wifi::{
    init,
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
        WifiState,
    },
    EspWifiInitFor,
};
use smoltcp::{
    iface::{Config as IfaceConfig, Interface, SocketSet},
    socket::tcp::{Socket as TcpSocket, SocketBuffer},
    time::Instant,
    wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address},
};

esp_app_desc!();

// ─── compile-time WiFi credentials ───────────────────────────────────────────
const WIFI_SSID: &str = env!("OTA_WIFI_SSID");
const WIFI_PSK: &str = env!("OTA_WIFI_PSK");

// ─── OTA server parameters ────────────────────────────────────────────────────
const OTA_TCP_PORT: u16 = 8080;
/// Maximum firmware image size accepted (4 MB).
const MAX_OTA_SIZE: usize = 4 * 1024 * 1024;

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

/// Return `true` if the first HTTP request line is `POST /ota HTTP/1.x`.
fn is_valid_ota_request(header_block: &[u8]) -> bool {
    let text = match str::from_utf8(header_block) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let first_line = text.lines().next().unwrap_or("").trim();
    // Accept both HTTP/1.0 and HTTP/1.1.
    first_line.starts_with("POST /ota HTTP/1.")
}

/// Extract `Content-Length` value from a raw HTTP header block.
fn parse_content_length(header: &[u8]) -> Option<usize> {
    let text = str::from_utf8(header).ok()?;
    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("content-length:") {
            return lower
                .split(':')
                .nth(1)?
                .trim()
                .parse()
                .ok();
        }
    }
    None
}

/// Find the byte offset immediately after `\r\n\r\n` (end of HTTP headers).
fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
}

// ─── current time helper (ms since boot, wrapping) ────────────────────────────
fn current_millis() -> u64 {
    // esp-hal provides `esp_hal::time::Instant`; convert to ms.
    esp_hal::time::Instant::now()
        .duration_since_epoch()
        .to_millis()
}

// ─── main ──────────────────────────────────────────────────────────────────────
#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // esp-wifi requires a hardware timer for its internal scheduler.
    let timg1 = TimerGroup::new(peripherals.TIMG1);
    let mut rng = Rng::new(peripherals.RNG);

    // Sample two 32-bit values before handing the RNG to esp-wifi, so that
    // the smoltcp TCP ISN seed is non-deterministic even after a cold boot.
    let random_seed: u64 = (rng.random() as u64) << 32 | rng.random() as u64;

    let init = init(
        EspWifiInitFor::Wifi,
        timg1.timer0,
        rng,
        peripherals.RADIO_CLK,
    )
    .expect("esp-wifi init failed");

    // Create WiFi station-mode device + controller.
    let (wifi_device, mut wifi_controller) =
        esp_wifi::wifi::new_with_mode(&init, peripherals.WIFI, WifiStaDevice)
            .expect("wifi new failed");

    // Configure the station with our SSID / PSK.
    wifi_controller
        .set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: WIFI_SSID.try_into().expect("SSID too long"),
            password: WIFI_PSK.try_into().expect("PSK too long"),
            ..Default::default()
        }))
        .expect("wifi set_configuration failed");

    wifi_controller.start().expect("wifi start failed");
    println!("[ota] WiFi started, connecting to \"{}\"…", WIFI_SSID);

    // Block until association succeeds.
    loop {
        match wifi_controller.connect() {
            Ok(()) => break,
            Err(e) => {
                println!("[ota] connect error: {:?} — retrying", e);
                blocking_delay_ms(1000);
            }
        }
    }
    println!("[ota] connected");

    // Wait for DHCP lease.
    loop {
        if matches!(wifi_controller.is_connected(), Ok(true)) {
            break;
        }
        blocking_delay_ms(200);
    }

    // ── smoltcp interface setup ────────────────────────────────────────────────
    let mac = wifi_device.mac_address();
    let ethernet_addr = EthernetAddress(mac);

    let mut iface_config = IfaceConfig::new(ethernet_addr.into());
    iface_config.random_seed = random_seed;

    let mut iface = Interface::new(iface_config, &mut &wifi_device, smoltcp_now());

    // Use DHCP-assigned address when available; placeholder until lease arrives.
    iface.update_ip_addrs(|addrs| {
        addrs
            .push(IpCidr::new(IpAddress::v4(0, 0, 0, 0), 0))
            .ok();
    });

    // ── TCP socket for OTA ─────────────────────────────────────────────────────
    let mut rx_buf = [0u8; 1536];
    let mut tx_buf = [0u8; 512];
    let tcp_rx = SocketBuffer::new(&mut rx_buf[..]);
    let tcp_tx = SocketBuffer::new(&mut tx_buf[..]);
    let ota_socket = TcpSocket::new(tcp_rx, tcp_tx);

    let mut socket_set_storage = [smoltcp::iface::SocketStorage::EMPTY; 2];
    let mut sockets = SocketSet::new(&mut socket_set_storage[..]);
    let ota_handle = sockets.add(ota_socket);

    // ── wait for DHCP then print IP ────────────────────────────────────────────
    let local_ip = wait_for_dhcp(&mut iface, &wifi_device, &mut sockets);
    println!(
        "[ota] IP address: {}  OTA port: {}",
        local_ip, OTA_TCP_PORT
    );
    println!("[ota] ready — waiting for firmware upload");
    println!(
        "[ota] upload with:  ./scripts/flash-esp32.sh <firmware> --ota {}",
        local_ip
    );

    // Listen for an incoming OTA connection.
    {
        let socket = sockets.get_mut::<TcpSocket>(ota_handle);
        socket.listen(OTA_TCP_PORT).expect("listen failed");
    }

    // ── main event loop ────────────────────────────────────────────────────────
    let mut firmware_buf: Vec<u8> = Vec::new();
    let mut header_buf: Vec<u8> = Vec::new();
    let mut expected_len: Option<usize> = None;
    let mut header_received = false;

    loop {
        let timestamp = smoltcp_now();
        iface.poll(timestamp, &mut &wifi_device, &mut sockets);

        let socket = sockets.get_mut::<TcpSocket>(ota_handle);

        if socket.is_active() && socket.may_recv() {
            let mut chunk = [0u8; 1024];
            match socket.recv_slice(&mut chunk) {
                Ok(0) => {}
                Ok(n) => {
                    if !header_received {
                        header_buf.extend_from_slice(&chunk[..n]);

                        if let Some(body_start) = header_end(&header_buf) {
                            // Validate HTTP method and path before doing anything else.
                            if !is_valid_ota_request(&header_buf[..body_start]) {
                                println!("[ota] ERROR: not a POST /ota request, rejecting");
                                socket.send_slice(b"HTTP/1.0 400 Bad Request\r\nContent-Length: 12\r\n\r\nBad Request\n").ok();
                                socket.close();
                                continue;
                            }

                            // Parse Content-Length from headers.
                            if let Some(cl) = parse_content_length(&header_buf[..body_start]) {
                                expected_len = Some(cl);
                                println!(
                                    "[ota] header received — expecting {} bytes of firmware",
                                    cl
                                );
                                if cl > MAX_OTA_SIZE {
                                    println!("[ota] ERROR: firmware too large ({}), rejecting", cl);
                                    socket.close();
                                    continue;
                                }
                                firmware_buf.reserve_exact(cl);
                            } else {
                                println!("[ota] ERROR: missing Content-Length header");
                                socket.close();
                                continue;
                            }

                            // Body bytes that arrived with the headers.
                            let body_bytes = &header_buf[body_start..];
                            firmware_buf.extend_from_slice(body_bytes);
                            header_received = true;
                            header_buf.clear();
                        }
                    } else {
                        firmware_buf.extend_from_slice(&chunk[..n]);
                    }

                    // Check if we have the complete image.
                    if let Some(expected) = expected_len {
                        if firmware_buf.len() >= expected {
                            firmware_buf.truncate(expected);
                            println!(
                                "[ota] received {} bytes — writing OTA slot…",
                                firmware_buf.len()
                            );

                            match write_ota_image(&firmware_buf) {
                                Ok(()) => {
                                    println!("[ota] write OK — flushing response then rebooting");
                                    socket
                                        .send_slice(b"HTTP/1.0 200 OK\r\nContent-Length: 7\r\n\r\nOTA OK\n")
                                        .ok();
                                    // Drive iface.poll() until the TX buffer drains so the
                                    // 200 response actually reaches the host before reset.
                                    // smoltcp only transmits frames inside poll(); a bare
                                    // blocking_delay_ms() would leave the data in RAM.
                                    let deadline = esp_hal::time::Instant::now()
                                        + esp_hal::time::Duration::from_millis(500);
                                    loop {
                                        iface.poll(smoltcp_now(), &mut &wifi_device, &mut sockets);
                                        let s = sockets.get_mut::<TcpSocket>(ota_handle);
                                        let drained = s.send_queue() == 0;
                                        let timed_out = esp_hal::time::Instant::now() >= deadline;
                                        if drained || timed_out {
                                            break;
                                        }
                                        // Short delay to avoid hammering the WiFi driver.
                                        blocking_delay_ms(5);
                                    }
                                    esp_hal::system::software_reset();
                                }
                                Err(e) => {
                                    println!("[ota] write FAILED: {:?}", e);
                                    socket
                                        .send_slice(
                                            b"HTTP/1.0 500 Internal Server Error\r\nContent-Length: 11\r\n\r\nOTA FAILED\n",
                                        )
                                        .ok();
                                    socket.close();
                                    // Reset state for next attempt.
                                    firmware_buf.clear();
                                    expected_len = None;
                                    header_received = false;
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    socket.close();
                    firmware_buf.clear();
                    expected_len = None;
                    header_received = false;
                }
            }
        }

        // Accept next connection after the previous one is closed.
        if !socket.is_open() {
            firmware_buf.clear();
            header_buf.clear();
            expected_len = None;
            header_received = false;
            socket.listen(OTA_TCP_PORT).ok();
            println!("[ota] ready — waiting for firmware upload");
        }
    }
}

// ─── OTA flash write ──────────────────────────────────────────────────────────

#[derive(Debug)]
enum OtaError {
    BeginFailed,
    WriteFailed,
    FinishFailed,
}

/// Write `image` to the inactive OTA slot and schedule it as the next boot.
fn write_ota_image(image: &[u8]) -> Result<(), OtaError> {
    let mut ota = esp_ota::OtaUpdate::begin().map_err(|_| OtaError::BeginFailed)?;

    const CHUNK: usize = 4096;
    let mut offset = 0;
    while offset < image.len() {
        let end = (offset + CHUNK).min(image.len());
        ota.write(&image[offset..end])
            .map_err(|_| OtaError::WriteFailed)?;
        offset = end;
        // Progress indicator every 64 KB.
        if offset % (64 * 1024) == 0 || offset == image.len() {
            println!("[ota] flashed {} / {} bytes", offset, image.len());
        }
    }

    ota.finish().map_err(|_| OtaError::FinishFailed)?;
    Ok(())
}

// ─── helpers ──────────────────────────────────────────────────────────────────

fn smoltcp_now() -> Instant {
    Instant::from_millis(current_millis() as i64)
}

/// Block-spin for approximately `ms` milliseconds.
fn blocking_delay_ms(ms: u32) {
    let target = esp_hal::time::Instant::now()
        + esp_hal::time::Duration::from_millis(ms as u64);
    while esp_hal::time::Instant::now() < target {}
}

/// Poll until a non-zero IP address is assigned (DHCP).  Returns the address.
fn wait_for_dhcp(
    iface: &mut Interface,
    device: &WifiDevice<WifiStaDevice>,
    sockets: &mut SocketSet<'_>,
) -> Ipv4Address {
    loop {
        iface.poll(smoltcp_now(), &mut &*device, sockets);

        let addr = iface.ipv4_addr();
        if let Some(ip) = addr {
            if !ip.is_unspecified() {
                return ip;
            }
        }

        blocking_delay_ms(100);
    }
}
