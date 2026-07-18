//! original-esp32-wifi-climate
//!
//! WiFi接続 → BME280温湿度読み取り → LCD1602表示 → Raspi IoTサーバーにHTTP POST
//! バックグラウンドでOTAサーバー（ポート8080）も稼働。
//!
//! ビルド設定 (WiFi 認証情報・サーバー情報・OTA トークン) は `.env` で管理します。
//! `.env.example` を `.env` にコピーして値を埋めてください (`.env` は gitignore 済み)。
//! build.rs が `.env` を読み込み、コンパイル時 env として注入します。
//! ```
//! cp .env.example .env
//! # .env を編集: WIFI_SSID / WIFI_PSK / RASPI_IP / DEVICE_ID / OTA_AUTH_TOKEN
//! cargo build --release
//! ```
//!
//! `.env` を使わず環境変数を直接渡すこともできます:
//! ```
//! WIFI_SSID=<ssid> WIFI_PSK=<psk> RASPI_IP=<ip> DEVICE_ID=<id> OTA_AUTH_TOKEN=<token> \
//!     cargo build --release
//! ```
//!
//! USBフラッシュ(初回):
//!   ./scripts/flash-esp32.sh firmware/original-esp32-wifi-climate
//!
//! OTAアップデート(2回目以降): 稼働中ファームに `POST /switch`
//!   (ヘッダ `X-OTA-Token: <OTA_AUTH_TOKEN>` が必須) を送ると updater へ退避し、
//!   updater の `POST /ota` で新ファームを受け取ります。

#![no_std]
#![no_main]

extern crate alloc;
use esp_alloc as _;

use alloc::vec::Vec;
use core::cell::RefCell;
use core::mem::MaybeUninit;
use core::fmt::Write as FmtWrite;
use core::str;

use embedded_hal::delay::DelayNs;
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
    socket::{dhcpv4, tcp::{Socket as TcpSocket, SocketBuffer, State as TcpState}},
    time::Instant as SmolInstant,
    wire::{EthernetAddress, IpCidr, Ipv4Address},
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
/// `POST /switch` を認可するための共有シークレット。updater 側の `OTA_AUTH_TOKEN`
/// と一致させる。空文字の場合は fail-closed で switch を常に拒否する。
const OTA_AUTH_TOKEN: &str = env!("OTA_AUTH_TOKEN");

const POST_INTERVAL_MS: u64 = 30_000;
const OTA_PORT: u16 = 8080;
const BME280_CHIP_ID_REG: u8 = 0xD0;
const BME280_CHIP_ID_VAL: u8 = 0x60;

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

fn is_switch_request(header: &[u8]) -> bool {
    str::from_utf8(header)
        .map(|s| s.lines().next().unwrap_or("").starts_with("POST /switch HTTP/1."))
        .unwrap_or(false)
}

/// リクエストヘッダに `X-OTA-Token: <OTA_AUTH_TOKEN>` が含まれるか検証する。
/// updater (original-esp32-ota-bringup) の認可方式と揃えている。
/// `OTA_AUTH_TOKEN` が空の場合は fail-closed で常に false。
fn header_has_valid_token(header: &[u8]) -> bool {
    if OTA_AUTH_TOKEN.is_empty() {
        return false;
    }
    let Ok(text) = str::from_utf8(header) else {
        return false;
    };
    for line in text.lines() {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("x-ota-token") && value.trim() == OTA_AUTH_TOKEN {
                return true;
            }
        }
    }
    false
}

// ─── OTA スロット切替 (otadata 直接書き換え) ──────────────────────────────────
// esp-ota-nostd の ota_reject は accept 済み (Valid) スロットを拒否できない
// ため、otadata に seq-1 の Valid エントリを直接書いて前スロット (app0 =
// updater) をアクティブにする。レイアウトとCRCは esp-ota-nostd/ESP-IDF 互換
// (実機の otadata で照合済み)。

const OTADATA_OFFSET: u32 = 0xe000;
const OTADATA_SECTOR: u32 = 0x1000;

/// ESP-IDF `esp_rom_crc32_le` 互換 (入力反転 + 出力反転の reflected CRC32)
fn esp_crc32_seq(seq: u32) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for b in seq.to_le_bytes() {
        crc ^= (!b) as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// /switch 受信を次回ブートへ伝えるフラグ (RTC RAM はリセット間で保持される)。
/// WiFi 稼働中のフラッシュ操作はハングするため、otadata の書き換えは
/// リブート直後の WiFi 初期化前に行う。
#[esp_hal::ram(rtc_fast, persistent)]
static mut SWITCH_MAGIC: u32 = 0;

const SWITCH_MAGIC_VALUE: u32 = 0x5357_4954; // "SWIT"

/// 起動失敗カウンタ (自動フォールバック用)。POST 成功に至らないまま
/// 再起動が繰り返された場合、不良ファームとみなして updater スロットへ
/// 自動退避する。電源断では RTC RAM が不定値になるため magic とペアで持ち、
/// magic 不一致ならカウンタ 0 から数え直す。
#[esp_hal::ram(rtc_fast, persistent)]
static mut BOOT_FAIL_MAGIC: u32 = 0;
#[esp_hal::ram(rtc_fast, persistent)]
static mut BOOT_FAIL_COUNT: u32 = 0;

const BOOT_FAIL_MAGIC_VALUE: u32 = 0x424F_4F54; // "BOOT"
/// この回数 POST 成功前の再起動が続いたら updater へ退避
const MAX_UNHEALTHY_BOOTS: u32 = 3;

fn read_boot_fail_count() -> u32 {
    unsafe {
        if core::ptr::read_volatile(core::ptr::addr_of!(BOOT_FAIL_MAGIC)) == BOOT_FAIL_MAGIC_VALUE {
            core::ptr::read_volatile(core::ptr::addr_of!(BOOT_FAIL_COUNT))
        } else {
            0
        }
    }
}

fn write_boot_fail_count(count: u32) {
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(BOOT_FAIL_MAGIC), BOOT_FAIL_MAGIC_VALUE);
        core::ptr::write_volatile(core::ptr::addr_of_mut!(BOOT_FAIL_COUNT), count);
    }
}

/// esp-storage は非整列バッファを 4KB のスタックバッファ経由で扱うため、
/// 32B エントリは 4B 整列させて直接書き込みパスを通す
#[repr(align(4))]
struct AlignedEntry([u8; 32]);

/// otadata の両セクタを seq-1 の Valid エントリで書き換える。
/// 成功時は新しい seq を返す。呼び出し後は software_reset すること。
fn switch_boot_slot(flash: &mut FlashStorage) -> Result<u32, ()> {
    use embedded_storage::nor_flash::{NorFlash as _, ReadNorFlash as _};

    let mut buf = AlignedEntry([0u8; 32]);
    let mut cur: Option<u32> = None;
    for i in 0..2u32 {
        if flash.read(OTADATA_OFFSET + i * OTADATA_SECTOR, &mut buf.0).is_err() {
            continue;
        }
        let s = u32::from_le_bytes(buf.0[0..4].try_into().unwrap());
        let crc = u32::from_le_bytes(buf.0[28..32].try_into().unwrap());
        if s != u32::MAX && crc == esp_crc32_seq(s) {
            cur = Some(cur.map_or(s, |p| p.max(s)));
        }
    }
    let cur = cur.ok_or(())?;
    if cur < 2 {
        // seq=1 の前は「エントリなし」になってしまうため切替不可
        return Err(());
    }
    let new_seq = cur - 1;

    let mut entry = AlignedEntry([0xFFu8; 32]);
    entry.0[0..4].copy_from_slice(&new_seq.to_le_bytes());
    entry.0[24..28].copy_from_slice(&2u32.to_le_bytes()); // EspOTAState::Valid
    entry.0[28..32].copy_from_slice(&esp_crc32_seq(new_seq).to_le_bytes());

    // 各 otadata セクタを erase→write→read検証。フラッシュ書き込みは稀に
    // 失敗するため、検証で不一致なら数回やり直す。両セクタが検証を通って
    // 初めて成功とする（中途半端な otadata で起動不能になるのを防ぐ）。
    for i in 0..2u32 {
        let off = OTADATA_OFFSET + i * OTADATA_SECTOR;
        let mut ok = false;
        for _ in 0..5 {
            if flash.erase(off, off + OTADATA_SECTOR).is_err() {
                continue;
            }
            if flash.write(off, &entry.0).is_err() {
                continue;
            }
            let mut back = AlignedEntry([0u8; 32]);
            if flash.read(off, &mut back.0).is_ok() && back.0 == entry.0 {
                ok = true;
                break;
            }
        }
        if !ok {
            return Err(());
        }
    }
    Ok(new_seq)
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
    rwdt: &mut esp_hal::rtc_cntl::Rwdt,
) -> Ipv4Address {
    let mut tick = 0u32;
    loop {
        rwdt.feed();
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
    // RWDT (60秒): panic ハンドラは halt するためリブートせず、自動フォール
    // バックのカウンタが増えない。ウォッチドッグで halt からも自動リブート
    // させる。全ブロッキングループで feed すること。
    let peripherals = esp_hal::init(
        esp_hal::Config::default().with_watchdog(
            esp_hal::config::WatchdogConfig::default().with_rwdt(
                esp_hal::config::WatchdogStatus::Enabled(
                    esp_hal::time::Duration::from_secs(60),
                ),
            ),
        ),
    );
    let mut rwdt = esp_hal::rtc_cntl::Rwdt::new();

    // 起動時の ota_accept は行わない:
    // - updater (ESP-IDF) が書く otadata は state=Undefined で、bootloader は
    //   rollback 無効のためそのまま起動できる (承認不要)
    // - ota_accept の write_ota_data が起動直後に LoadProhibited パニックを
    //   起こすケースが実機であり (esp-storage write 経由)、回避する

    // 前回の /switch 要求の処理: WiFi 初期化前のこの時点ならフラッシュ書き込みが
    // 安全なので、ここで otadata を書き換えて updater スロットへ切り替える
    let magic = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SWITCH_MAGIC)) };
    if magic == SWITCH_MAGIC_VALUE {
        unsafe { core::ptr::write_volatile(core::ptr::addr_of_mut!(SWITCH_MAGIC), 0) };
        write_boot_fail_count(0); // 次に起動するファームには新しい猶予を与える
        println!("[switch] boot-time otadata rewrite for updater slot");
        let mut flash = FlashStorage::new();
        match switch_boot_slot(&mut flash) {
            Ok(seq) => println!("[switch] otadata -> seq={} — rebooting into updater", seq),
            Err(()) => println!("[switch] otadata rewrite failed — booting normally"),
        }
        esp_hal::system::software_reset();
    }

    // 自動フォールバック: POST 成功 (健全性確認) に至らないまま再起動が
    // MAX_UNHEALTHY_BOOTS 回続いたら、不良ファームとみなして updater へ退避。
    // 起動できないファームを OTA してしまってもシリアル接続なしで復旧できる。
    // カウンタは最初の POST 成功時にクリアされる (メインループ内)。
    let unhealthy = read_boot_fail_count();
    if unhealthy >= MAX_UNHEALTHY_BOOTS {
        write_boot_fail_count(0);
        println!(
            "[fallback] {} consecutive boots without a successful POST — switching to updater",
            unhealthy
        );
        let mut flash = FlashStorage::new();
        if switch_boot_slot(&mut flash).is_ok() {
            esp_hal::system::software_reset();
        }
        println!("[fallback] otadata rewrite failed — continuing normal boot");
    } else {
        write_boot_fail_count(unhealthy + 1);
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
    // モデムスリープ無効化: 省電力中は着信 (ping/SYN) をほぼ取りこぼすため、
    // OTAサーバーとして常時受信できるようにする (AC給電前提)
    wifi_ctrl
        .set_power_saving(esp_wifi::config::PowerSaveMode::None)
        .expect("set_power_saving");
    println!("[wifi] connecting to \"{}\"…", WIFI_SSID);

    // 接続 → link up 完了まで。WPA2鍵交換が電波状況で失敗したまま
    // 固まることがあるため、10秒で諦めて connect からやり直す
    'wifi: loop {
        loop {
            rwdt.feed();
            match wifi_ctrl.connect() {
                Ok(()) => break,
                Err(e) => { println!("[wifi] retry: {:?}", e); blocking_delay_ms(1000); }
            }
        }
        println!("[wifi] associated");

        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            rwdt.feed();
            if matches!(wifi_ctrl.is_connected(), Ok(true)) {
                break 'wifi;
            }
            blocking_delay_ms(200);
        }
        println!("[wifi] link timeout — reconnecting");
        wifi_ctrl.disconnect().ok();
        blocking_delay_ms(500);
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

    let local_ip = wait_for_dhcp(&mut iface, &mut wifi_dev, &mut sockets, dhcp_h, &mut rwdt);
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
    let mut hdr_buf: Vec<u8> = Vec::new();

    // ─── HTTP POST state ──────────────────────────────────────────────────────
    let mut last_post_ms: u64 = 0;
    let mut http_state = HttpState::Idle;
    let mut http_req_buf = [0u8; 512];
    let mut http_req_len = 0usize;
    let mut http_req_sent = 0usize;
    // 初回 POST 成功で true → 自動フォールバックのカウンタをクリア
    let mut health_confirmed = false;

    println!("[main] ready — posting every {}s", POST_INTERVAL_MS / 1000);

    loop {
        let now_ms = millis();
        rwdt.feed();
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

        // ── 管理サーバー (port 8080) ──────────────────────────────────────────
        // POST /switch: 自スロットを invalid 化して再起動 → app0 の updater が
        // 起動し、そちらの POST /ota で新ファームを受ける。
        // (このファーム内でのフラッシュ書き込みは esp-wifi と競合してハングする
        //  ため、OTA 本体は ESP-IDF ベースの updater に任せる)
        {
            let mut switch_requested = false;
            {
                let s = sockets.get_mut::<TcpSocket>(ota_h);
                if s.is_active() && s.may_recv() {
                    let mut chunk = [0u8; 1024];
                    match s.recv_slice(&mut chunk) {
                        Ok(0) => {}
                        Ok(n) => {
                            hdr_buf.extend_from_slice(&chunk[..n]);
                            if let Some(body_start) = header_end(&hdr_buf) {
                                let head = &hdr_buf[..body_start];
                                if is_switch_request(head) {
                                    if header_has_valid_token(head) {
                                        s.send_slice(b"HTTP/1.0 200 OK\r\nContent-Length: 20\r\n\r\nswitching to updater").ok();
                                        switch_requested = true;
                                    } else {
                                        // 認可なしの /switch は拒否 (同一 LAN からの
                                        // 無認証ファーム上書きを防ぐ)。
                                        s.send_slice(b"HTTP/1.0 401 Unauthorized\r\nContent-Length: 12\r\n\r\nunauthorized").ok();
                                        s.close();
                                    }
                                } else if is_valid_ota_request(head) {
                                    s.send_slice(b"HTTP/1.0 409 Conflict\r\nContent-Length: 33\r\n\r\nPOST /switch first, then use /ota").ok();
                                    s.close();
                                } else {
                                    s.send_slice(b"HTTP/1.0 400 Bad Request\r\nContent-Length: 0\r\n\r\n").ok();
                                    s.close();
                                }
                                hdr_buf.clear();
                            }
                        }
                        Err(_) => {
                            s.close();
                            hdr_buf.clear();
                        }
                    }
                }
            }

            if switch_requested {
                println!("[switch] flushing response, then rebooting to rewrite otadata…");
                // 200 応答をフラッシュしてから再起動する
                let dl = Instant::now() + Duration::from_millis(500);
                loop {
                    iface.poll(smol_now(), &mut wifi_dev, &mut sockets);
                    let s2 = sockets.get_mut::<TcpSocket>(ota_h);
                    if s2.send_queue() == 0 || Instant::now() >= dl { break; }
                    blocking_delay_ms(5);
                }
                // WiFi 稼働中のフラッシュ操作はハングするため、ここでは
                // RTC RAM にフラグを立てて再起動し、次回ブート冒頭
                // (WiFi 初期化前) で otadata を書き換える
                unsafe {
                    core::ptr::write_volatile(
                        core::ptr::addr_of_mut!(SWITCH_MAGIC),
                        SWITCH_MAGIC_VALUE,
                    );
                }
                esp_hal::system::software_reset();
            }

            let s = sockets.get_mut::<TcpSocket>(ota_h);
            // Peer closed without a request (e.g. port scan): CLOSE_WAIT never
            // reaches !is_open(), so close our side too or listen stops forever
            if s.state() == TcpState::CloseWait {
                s.close();
            }
            if !s.is_open() {
                hdr_buf.clear();
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
                                if !health_confirmed {
                                    // 初回 POST 成功 = このファームは健全。
                                    // 自動フォールバックのカウンタをクリアする
                                    health_confirmed = true;
                                    write_boot_fail_count(0);
                                    println!("[fallback] boot marked healthy");
                                }
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
                } else if s.state() == TcpState::CloseWait {
                    // Server sent FIN with no (remaining) data: close or we
                    // stay in Receiving forever and posting stops
                    s.close();
                    http_state = HttpState::Done;
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
