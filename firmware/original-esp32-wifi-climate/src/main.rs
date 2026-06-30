//! original-esp32-wifi-climate
//!
//! WiFi接続 → BME280温湿度読み取り → LCD1602表示 → Raspi IoTサーバーにHTTP POST
//! バックグラウンドでOTAサーバー（ポート8080）も稼働。
//!
//! ビルド:
//! ```
//! WIFI_SSID=TP_Link_Living WIFI_PSK=20210722 RASPI_IP=192.168.1.220 DEVICE_ID=living-room \
//!     cargo build --release
//! ```
//!
//! USBフラッシュ(初回):
//!   WIFI_SSID=... WIFI_PSK=... RASPI_IP=... DEVICE_ID=... \
//!       ./scripts/flash-esp32.sh firmware/original-esp32-wifi-climate
//!
//! OTAアップデート(2回目以降):
//!   ./scripts/flash-esp32.sh firmware/original-esp32-wifi-climate --ota <ESP32のIP>

#![no_std]
#![no_main]

extern crate alloc;
use esp_alloc as _;

use alloc::vec::Vec;
use core::cell::RefCell;
use core::mem::MaybeUninit;
use core::convert::Infallible;
use core::fmt::Write as FmtWrite;
use core::str;

use embedded_hal::delay::DelayNs;
use embedded_io_async::{ErrorType, Read as AsyncRead};
use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    main,
    rng::Rng,
    time::{Duration, Instant},
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_storage::FlashStorage;
use esp_wifi::{
    init,
    wifi::{ClientConfiguration, Configuration, WifiDevice},
};
use hal_api::display::{TextDisplay16x2, TextFrame16x2};
use heapless::String as HString;
use platform_esp32::{
    bme280::{BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY, Bme280Config, Bme280Sensor},
    i2c::Esp32I2c,
    lcd1602::{LCD1602_ADDRESS_PRIMARY, Lcd1602Config, Lcd1602Display},
    shared_i2c::SharedI2cBus,
};
use smoltcp::{
    iface::{Config as IfaceConfig, Interface, SocketSet},
    socket::{dhcpv4, tcp::{Socket as TcpSocket, SocketBuffer}},
    time::Instant as SmolInstant,
    wire::{EthernetAddress, IpCidr, Ipv4Address, Ipv4Cidr},
};

esp_bootloader_esp_idf::esp_app_desc!();
#[used]
static _KEEP_APP_DESC: &esp_bootloader_esp_idf::EspAppDesc = &ESP_APP_DESC;

// ─── ヒープ (esp_wifi の malloc が使うため esp_wifi::init() 前に初期化が必要) ─────
const HEAP_SIZE: usize = 72 * 1024;
static mut HEAP_MEM: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

// ─── compile-time config ──────────────────────────────────────────────────────
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PSK: &str = env!("WIFI_PSK");
const RASPI_IP: &str = env!("RASPI_IP");
const RASPI_PORT: u16 = 8000;
const DEVICE_ID: &str = env!("DEVICE_ID");

const POST_INTERVAL_MS: u64 = 30_000;
const OTA_PORT: u16 = 8080;
const MAX_OTA_SIZE: usize = 4 * 1024 * 1024;
const BME280_CHIP_ID_REG: u8 = 0xD0;
const BME280_CHIP_ID_VAL: u8 = 0x60;

// ─── OTA async reader ─────────────────────────────────────────────────────────
struct SliceAsyncReader<'a> {
    data: &'a [u8],
    cursor: usize,
}

impl<'a> SliceAsyncReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, cursor: 0 }
    }
}

impl ErrorType for SliceAsyncReader<'_> {
    type Error = Infallible;
}

impl AsyncRead for SliceAsyncReader<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Infallible> {
        if self.cursor >= self.data.len() {
            return Ok(0);
        }
        let n = (self.data.len() - self.cursor).min(buf.len());
        buf[..n].copy_from_slice(&self.data[self.cursor..self.cursor + n]);
        self.cursor += n;
        Ok(n)
    }
}

// ─── delay ────────────────────────────────────────────────────────────────────
#[derive(Default, Clone, Copy)]
struct MonotonicDelay;

impl DelayNs for MonotonicDelay {
    fn delay_ns(&mut self, ns: u32) {
        let end = Instant::now() + Duration::from_micros(ns.div_ceil(1000) as u64);
        while Instant::now() < end {}
    }
    fn delay_us(&mut self, us: u32) {
        let end = Instant::now() + Duration::from_micros(us as u64);
        while Instant::now() < end {}
    }
    fn delay_ms(&mut self, ms: u32) {
        let end = Instant::now() + Duration::from_millis(ms as u64);
        while Instant::now() < end {}
    }
}

// ─── helpers ──────────────────────────────────────────────────────────────────
fn millis() -> u64 {
    Instant::now().duration_since_epoch().as_millis()
}

fn smol_now() -> SmolInstant {
    SmolInstant::from_millis(millis() as i64)
}

fn blocking_delay_ms(ms: u32) {
    let end = Instant::now() + Duration::from_millis(ms as u64);
    while Instant::now() < end {}
}

/// i32の絶対値をu32として返す (i32::MIN未使用を前提とした安全実装)
fn abs_i32(x: i32) -> u32 {
    if x >= 0 { x as u32 } else { (!(x as u32)).wrapping_add(1) }
}

fn detect_bme280_address<B>(bus: &mut B) -> u8
where
    B: hal_api::i2c::I2cBus<Error = hal_api::error::I2cError>,
{
    for addr in [BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY] {
        let mut buf = [0u8; 1];
        if let Ok(()) = bus.write_read(addr, &[BME280_CHIP_ID_REG], &mut buf) {
            if buf[0] == BME280_CHIP_ID_VAL {
                println!("[bme280] found at 0x{:02x}", addr);
                return addr;
            }
        }
    }
    BME280_ADDRESS_PRIMARY
}

fn parse_ipv4(s: &str) -> Option<Ipv4Address> {
    let mut octets = [0u8; 4];
    let mut idx = 0usize;
    let mut acc: u16 = 0;
    let mut dots = 0u8;
    for b in s.bytes() {
        match b {
            b'0'..=b'9' => {
                acc = acc * 10 + (b - b'0') as u16;
                if acc > 255 { return None; }
            }
            b'.' if idx < 3 => {
                octets[idx] = acc as u8;
                idx += 1;
                dots += 1;
                acc = 0;
            }
            _ => return None,
        }
    }
    if dots != 3 { return None; }
    octets[3] = acc as u8;
    Some(Ipv4Address::new(octets[0], octets[1], octets[2], octets[3]))
}

fn is_valid_ota_request(header: &[u8]) -> bool {
    str::from_utf8(header)
        .map(|s| s.lines().next().unwrap_or("").starts_with("POST /ota HTTP/1."))
        .unwrap_or(false)
}

fn parse_content_length(header: &[u8]) -> Option<usize> {
    let text = str::from_utf8(header).ok()?;
    for line in text.lines() {
        let low: HString<32> = {
            let mut s: HString<32> = HString::new();
            for c in line.chars().take(32) {
                let _ = s.push(c.to_ascii_lowercase());
            }
            s
        };
        if low.starts_with("content-length:") {
            return low["content-length:".len()..].trim().parse().ok();
        }
    }
    None
}

fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
}

/// JSONボディを整数演算で構築
fn build_json(buf: &mut [u8], device_id: &str, temp_cc: i32, hum_cp: u32) -> usize {
    let mut pos = 0usize;

    fn push(buf: &mut [u8], pos: &mut usize, s: &[u8]) {
        for &b in s {
            if *pos < buf.len() { buf[*pos] = b; *pos += 1; }
        }
    }

    fn push_u32(buf: &mut [u8], pos: &mut usize, mut n: u32) {
        let mut tmp = [0u8; 10];
        let mut len = 0;
        if n == 0 { push(buf, pos, b"0"); return; }
        while n > 0 { tmp[len] = b'0' + (n % 10) as u8; n /= 10; len += 1; }
        tmp[..len].reverse();
        push(buf, pos, &tmp[..len]);
    }

    fn push_frac2(buf: &mut [u8], pos: &mut usize, frac: u32) {
        let d1 = b'0' + (frac / 10) as u8;
        let d2 = b'0' + (frac % 10) as u8;
        if *pos < buf.len() { buf[*pos] = d1; *pos += 1; }
        if *pos < buf.len() { buf[*pos] = d2; *pos += 1; }
    }

    let t_abs = abs_i32(temp_cc);
    push(buf, &mut pos, b"{\"device_id\":\"");
    push(buf, &mut pos, device_id.as_bytes());
    push(buf, &mut pos, b"\",\"temperature\":");
    if temp_cc < 0 { push(buf, &mut pos, b"-"); }
    push_u32(buf, &mut pos, t_abs / 100);
    push(buf, &mut pos, b".");
    push_frac2(buf, &mut pos, t_abs % 100);
    push(buf, &mut pos, b",\"humidity\":");
    push_u32(buf, &mut pos, hum_cp / 100);
    push(buf, &mut pos, b".");
    push_frac2(buf, &mut pos, hum_cp % 100);
    push(buf, &mut pos, b"}");

    pos
}

/// HTTP POSTリクエストを構築
fn build_http_post(buf: &mut [u8], host: &str, json: &[u8]) -> usize {
    let mut pos = 0usize;

    fn push(buf: &mut [u8], pos: &mut usize, s: &[u8]) {
        for &b in s {
            if *pos < buf.len() { buf[*pos] = b; *pos += 1; }
        }
    }

    fn push_usize(buf: &mut [u8], pos: &mut usize, mut n: usize) {
        let mut tmp = [0u8; 10];
        let mut len = 0;
        if n == 0 { push(buf, pos, b"0"); return; }
        while n > 0 { tmp[len] = b'0' + (n % 10) as u8; n /= 10; len += 1; }
        tmp[..len].reverse();
        push(buf, pos, &tmp[..len]);
    }

    push(buf, &mut pos, b"POST /api/sensors/reading HTTP/1.0\r\nHost: ");
    push(buf, &mut pos, host.as_bytes());
    push(buf, &mut pos, b"\r\nContent-Type: application/json\r\nContent-Length: ");
    push_usize(buf, &mut pos, json.len());
    push(buf, &mut pos, b"\r\nConnection: close\r\n\r\n");
    push(buf, &mut pos, json);

    pos
}

/// 温湿度をLCDフレームに描画
fn make_lcd_frame(temp_cc: i32, hum_cp: u32) -> TextFrame16x2 {
    let t_abs = abs_i32(temp_cc);
    let h_abs = hum_cp;

    let mut line1: HString<16> = HString::new();
    let mut line2: HString<16> = HString::new();

    let t_sign = if temp_cc < 0 { "-" } else { "" };
    let _ = write!(line1, "Temp:{}{}.{:02}C", t_sign, t_abs / 100, t_abs % 100);
    let _ = write!(line2, "Hum:  {}.{:02}%", h_abs / 100, h_abs % 100);

    TextFrame16x2::from_lines(&line1, &line2)
}

// ─── DHCP wait ────────────────────────────────────────────────────────────────
fn wait_for_dhcp(
    iface: &mut Interface,
    device: &mut WifiDevice<'_>,
    sockets: &mut SocketSet<'_>,
    dhcp_h: smoltcp::iface::SocketHandle,
) -> Ipv4Address {
    let mut tick = 0u32;
    loop {
        iface.poll(smol_now(), device, sockets);

        if let Some(event) = sockets.get_mut::<dhcpv4::Socket>(dhcp_h).poll() {
            if let dhcpv4::Event::Configured(config) = event {
                let ip = config.address.address();
                iface.update_ip_addrs(|addrs| {
                    addrs.clear();
                    addrs.push(IpCidr::Ipv4(config.address)).ok();
                });
                if let Some(router) = config.router {
                    iface.routes_mut().add_default_ipv4_route(router).ok();
                    println!("[dhcp] gateway={}", router);
                }
                return ip;
            }
        }

        blocking_delay_ms(100);
        tick += 1;
        if tick % 20 == 0 {
            println!("[dhcp] waiting... {}s", tick / 10);
        }
    }
}

// ─── HTTP state machine ───────────────────────────────────────────────────────
#[derive(Clone, Copy)]
enum HttpState {
    Idle,
    Connecting,
    Sending,
    Receiving,
    Done,
}

// ─── main ──────────────────────────────────────────────────────────────────────
#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // OTA スロット承認
    {
        let mut flash = FlashStorage::new();
        if esp_ota_nostd::ota_accept(&mut flash).is_ok() {
            println!("[ota] slot accepted");
        }
    }

    // ─── ヒープ初期化 (esp_wifi::init() 内の malloc が使うため最初に行う) ──────
    unsafe {
        esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
            HEAP_MEM.as_mut_ptr() as *mut u8,
            HEAP_SIZE,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }

    // ─── WiFi ─────────────────────────────────────────────────────────────────
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);
    let seed: u64 = (rng.random() as u64) << 32 | rng.random() as u64;

    let wifi_init = init(timg0.timer0, rng, peripherals.RADIO_CLK).expect("esp-wifi init");
    let (mut wifi_ctrl, interfaces) =
        esp_wifi::wifi::new(&wifi_init, peripherals.WIFI).expect("wifi new");
    let mut wifi_dev = interfaces.sta;

    wifi_ctrl
        .set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: WIFI_SSID.try_into().expect("ssid"),
            password: WIFI_PSK.try_into().expect("psk"),
            ..Default::default()
        }))
        .expect("set_configuration");
    wifi_ctrl.start().expect("wifi start");
    println!("[wifi] connecting to \"{}\"…", WIFI_SSID);

    loop {
        match wifi_ctrl.connect() {
            Ok(()) => break,
            Err(e) => { println!("[wifi] retry: {:?}", e); blocking_delay_ms(1000); }
        }
    }
    println!("[wifi] associated");

    // Link が完全に確立されるまで待機（DHCP に必要）
    loop {
        if matches!(wifi_ctrl.is_connected(), Ok(true)) {
            break;
        }
        blocking_delay_ms(200);
    }
    println!("[wifi] link up");

    // ─── smoltcp ──────────────────────────────────────────────────────────────
    let mac = wifi_dev.mac_address();
    let mut iface_cfg = IfaceConfig::new(EthernetAddress(mac).into());
    iface_cfg.random_seed = seed;
    let mut iface = Interface::new(iface_cfg, &mut wifi_dev, smol_now());

    // ─── TCP + DHCP sockets ───────────────────────────────────────────────────
    let mut ota_rx = [0u8; 1536];
    let mut ota_tx = [0u8; 512];
    let mut http_rx = [0u8; 512];
    let mut http_tx = [0u8; 1024];
    let ota_sock = TcpSocket::new(
        SocketBuffer::new(&mut ota_rx[..]),
        SocketBuffer::new(&mut ota_tx[..]),
    );
    let http_sock = TcpSocket::new(
        SocketBuffer::new(&mut http_rx[..]),
        SocketBuffer::new(&mut http_tx[..]),
    );
    let dhcp_sock = dhcpv4::Socket::new();
    let mut ss = [smoltcp::iface::SocketStorage::EMPTY; 4];
    let mut sockets = SocketSet::new(&mut ss[..]);
    let ota_h = sockets.add(ota_sock);
    let http_h = sockets.add(http_sock);
    let dhcp_h = sockets.add(dhcp_sock);

    let local_ip = wait_for_dhcp(&mut iface, &mut wifi_dev, &mut sockets, dhcp_h);
    println!("[wifi] IP={}  OTA port={}", local_ip, OTA_PORT);

    let raspi_ip = parse_ipv4(RASPI_IP).expect("bad RASPI_IP");

    // ─── I2C + BME280 + LCD1602 (WiFi起動後に初期化) ──────────────────────────
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22);
    let mut raw_i2c = Esp32I2c::new(i2c);
    let bme280_addr = detect_bme280_address(&mut raw_i2c);
    let shared_bus = RefCell::new(raw_i2c);

    let mut sensor = Bme280Sensor::new_with_config(
        SharedI2cBus::new(&shared_bus),
        Bme280Config { address: bme280_addr, ..Bme280Config::default() },
    );
    let mut display = Lcd1602Display::new_with_config(
        SharedI2cBus::new(&shared_bus),
        MonotonicDelay,
        Lcd1602Config { address: LCD1602_ADDRESS_PRIMARY, ..Lcd1602Config::default() },
    );

    {
        let mut ready_line: HString<16> = HString::new();
        let _ = write!(ready_line, "IP:{}", local_ip);
        let frame = TextFrame16x2::from_lines("IoT Ready", &ready_line);
        let _ = display.render(&frame);
    }

    sockets.get_mut::<TcpSocket>(ota_h).listen(OTA_PORT).expect("ota listen");

    // ─── OTA state ────────────────────────────────────────────────────────────
    let mut fw_buf: Vec<u8> = Vec::new();
    let mut hdr_buf: Vec<u8> = Vec::new();
    let mut expected_len: Option<usize> = None;
    let mut hdr_received = false;

    // ─── HTTP POST state ──────────────────────────────────────────────────────
    let mut last_post_ms: u64 = 0;
    let mut http_state = HttpState::Idle;
    let mut http_req_buf = [0u8; 512];
    let mut http_req_len = 0usize;
    let mut http_req_sent = 0usize;

    println!("[main] ready — posting every {}s", POST_INTERVAL_MS / 1000);

    loop {
        let now_ms = millis();
        iface.poll(smol_now(), &mut wifi_dev, &mut sockets);

        // ── DHCP renewal ──────────────────────────────────────────────────────
        if let Some(event) = sockets.get_mut::<dhcpv4::Socket>(dhcp_h).poll() {
            match event {
                dhcpv4::Event::Configured(config) => {
                    iface.update_ip_addrs(|addrs| {
                        addrs.clear();
                        addrs.push(IpCidr::Ipv4(config.address)).ok();
                    });
                    if let Some(router) = config.router {
                        iface.routes_mut().add_default_ipv4_route(router).ok();
                    }
                }
                dhcpv4::Event::Deconfigured => {
                    iface.update_ip_addrs(|addrs| addrs.clear());
                    println!("[dhcp] deconfigured");
                }
            }
        }

        // ── OTA server ────────────────────────────────────────────────────────
        {
            let s = sockets.get_mut::<TcpSocket>(ota_h);
            if s.is_active() && s.may_recv() {
                let mut chunk = [0u8; 1024];
                match s.recv_slice(&mut chunk) {
                    Ok(0) => {}
                    Ok(n) => {
                        if !hdr_received {
                            hdr_buf.extend_from_slice(&chunk[..n]);
                            if let Some(body_start) = header_end(&hdr_buf) {
                                if !is_valid_ota_request(&hdr_buf[..body_start]) {
                                    s.send_slice(b"HTTP/1.0 400 Bad Request\r\nContent-Length: 0\r\n\r\n").ok();
                                    s.close();
                                } else if let Some(cl) = parse_content_length(&hdr_buf[..body_start]) {
                                    if cl > MAX_OTA_SIZE {
                                        println!("[ota] too large: {}", cl);
                                        s.close();
                                    } else {
                                        expected_len = Some(cl);
                                        fw_buf.reserve_exact(cl);
                                        fw_buf.extend_from_slice(&hdr_buf[body_start..]);
                                        hdr_received = true;
                                        hdr_buf.clear();
                                        println!("[ota] expecting {} B", cl);
                                    }
                                } else {
                                    s.close();
                                }
                            }
                        } else {
                            fw_buf.extend_from_slice(&chunk[..n]);
                        }
                        if let Some(exp) = expected_len {
                            if fw_buf.len() >= exp {
                                fw_buf.truncate(exp);
                                println!("[ota] writing {} B…", fw_buf.len());
                                let mut flash = FlashStorage::new();
                                let mut reader = SliceAsyncReader::new(&fw_buf);
                                let ok = embassy_futures::block_on(
                                    esp_ota_nostd::ota_begin(&mut flash, &mut reader, |_| {}),
                                ).is_ok();
                                if ok {
                                    s.send_slice(b"HTTP/1.0 200 OK\r\nContent-Length: 7\r\n\r\nOTA OK\n").ok();
                                    let dl = Instant::now() + Duration::from_millis(500);
                                    loop {
                                        iface.poll(smol_now(), &mut wifi_dev, &mut sockets);
                                        let s2 = sockets.get_mut::<TcpSocket>(ota_h);
                                        if s2.send_queue() == 0 || Instant::now() >= dl { break; }
                                        blocking_delay_ms(5);
                                    }
                                    esp_hal::system::software_reset();
                                } else {
                                    s.send_slice(b"HTTP/1.0 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n").ok();
                                    s.close();
                                    fw_buf.clear(); expected_len = None; hdr_received = false;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        s.close();
                        fw_buf.clear(); expected_len = None; hdr_received = false;
                    }
                }
            }
            if !s.is_open() {
                fw_buf.clear(); hdr_buf.clear(); expected_len = None; hdr_received = false;
                s.listen(OTA_PORT).ok();
            }
        }

        // ── センサー読み取り & HTTP POST ─────────────────────────────────────
        match http_state {
            HttpState::Idle => {
                if now_ms.saturating_sub(last_post_ms) >= POST_INTERVAL_MS {
                    match hal_api::sensor::EnvSensor::read(&mut sensor) {
                        Ok(reading) => {
                            let temp_cc = reading.temperature_centi_celsius;
                            let hum_cp = reading.humidity_centi_percent;
                            let t_abs = abs_i32(temp_cc);
                            println!(
                                "[sensor] {}{}.{:02}°C  {}.{:02}%",
                                if temp_cc < 0 { "-" } else { "" },
                                t_abs / 100, t_abs % 100,
                                hum_cp / 100, hum_cp % 100,
                            );

                            // LCD更新
                            let frame = make_lcd_frame(temp_cc, hum_cp);
                            let _ = display.render(&frame);

                            // HTTPリクエスト構築
                            let mut json_buf = [0u8; 128];
                            let json_len = build_json(&mut json_buf, DEVICE_ID, temp_cc, hum_cp);
                            http_req_len = build_http_post(
                                &mut http_req_buf,
                                RASPI_IP,
                                &json_buf[..json_len],
                            );
                            http_req_sent = 0;
                            http_state = HttpState::Connecting;
                        }
                        Err(e) => {
                            println!("[sensor] error: {:?}", e);
                            last_post_ms = now_ms;
                        }
                    }
                }
            }

            HttpState::Connecting => {
                let s = sockets.get_mut::<TcpSocket>(http_h);
                if !s.is_open() {
                    let local_port = 49152u16.wrapping_add((millis() % 16384) as u16);
                    match s.connect(iface.context(), (raspi_ip, RASPI_PORT), local_port) {
                        Ok(()) => {
                            println!("[http] connecting {}:{}…", RASPI_IP, RASPI_PORT);
                            http_state = HttpState::Sending;
                        }
                        Err(e) => {
                            println!("[http] connect err: {:?}", e);
                            http_state = HttpState::Done;
                        }
                    }
                }
            }

            HttpState::Sending => {
                let s = sockets.get_mut::<TcpSocket>(http_h);
                if s.may_send() && http_req_sent < http_req_len {
                    match s.send_slice(&http_req_buf[http_req_sent..http_req_len]) {
                        Ok(n) => { http_req_sent += n; }
                        Err(_) => { s.close(); http_state = HttpState::Done; }
                    }
                } else if http_req_sent >= http_req_len {
                    http_state = HttpState::Receiving;
                }
            }

            HttpState::Receiving => {
                let s = sockets.get_mut::<TcpSocket>(http_h);
                if s.may_recv() {
                    let mut resp = [0u8; 64];
                    match s.recv_slice(&mut resp) {
                        Ok(n) if n > 0 => {
                            let status = str::from_utf8(&resp[..n]).unwrap_or("");
                            if status.contains("201") || status.contains("200") {
                                println!("[http] POST ok");
                            } else {
                                let end = status.len().min(40);
                                println!("[http] resp: {}", &status[..end]);
                            }
                            s.close();
                            http_state = HttpState::Done;
                        }
                        Ok(_) => {}
                        Err(_) => { s.close(); http_state = HttpState::Done; }
                    }
                } else if !s.is_open() {
                    http_state = HttpState::Done;
                }
            }

            HttpState::Done => {
                last_post_ms = now_ms;
                http_state = HttpState::Idle;
            }
        }

        blocking_delay_ms(10);
    }
}
