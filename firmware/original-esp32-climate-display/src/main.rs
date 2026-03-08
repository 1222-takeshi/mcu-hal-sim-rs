#![no_std]
#![no_main]

use core::cell::RefCell;

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use embedded_hal::delay::DelayNs;
use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::{Duration, Instant},
};
use esp_println::println;
use platform_esp32::bme280::{BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY, Bme280Sensor};
use platform_esp32::i2c::Esp32I2c;
use platform_esp32::lcd1602::{LCD1602_ADDRESS_PRIMARY, Lcd1602Display};
use platform_esp32::shared_i2c::SharedI2cBus;

esp_bootloader_esp_idf::esp_app_desc!();

const I2C_SDA_GPIO: u8 = 21;
const I2C_SCL_GPIO: u8 = 22;
const REFRESH_PERIOD_TICKS: u32 = 10;
const LOOP_DELAY_MS: u32 = 100;
const BME280_CHIP_ID_REGISTER: u8 = 0xD0;
const BME280_CHIP_ID_VALUE: u8 = 0x60;

fn every_nth(value: u32, period: u32) -> bool {
    period != 0 && value % period == 0
}

#[derive(Default, Clone, Copy)]
struct MonotonicDelay;

impl MonotonicDelay {
    fn wait(duration: Duration) {
        let start = Instant::now();
        while start.elapsed() < duration {}
    }
}

impl DelayNs for MonotonicDelay {
    fn delay_ns(&mut self, ns: u32) {
        Self::wait(Duration::from_micros(ns.div_ceil(1000) as u64));
    }

    fn delay_us(&mut self, us: u32) {
        Self::wait(Duration::from_micros(us as u64));
    }

    fn delay_ms(&mut self, ms: u32) {
        Self::wait(Duration::from_millis(ms as u64));
    }
}

fn detect_bme280_address<B>(bus: &mut B) -> u8
where
    B: hal_api::i2c::I2cBus<Error = hal_api::error::I2cError>,
{
    for address in [BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY] {
        let mut chip_id = [0u8; 1];
        match bus.write_read(address, &[BME280_CHIP_ID_REGISTER], &mut chip_id) {
            Ok(()) if chip_id[0] == BME280_CHIP_ID_VALUE => {
                println!(
                    "BME280 probe: detected sensor at 0x{:02x} (chip-id=0x{:02x})",
                    address, chip_id[0]
                );
                return address;
            }
            Ok(()) => {
                println!(
                    "BME280 probe: address 0x{:02x} responded with unexpected chip-id=0x{:02x}",
                    address, chip_id[0]
                );
            }
            Err(error) => {
                println!("BME280 probe: address 0x{:02x} failed: {:?}", address, error);
            }
        }
    }

    println!(
        "BME280 probe: falling back to default address 0x{:02x}",
        BME280_ADDRESS_PRIMARY
    );
    BME280_ADDRESS_PRIMARY
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO21)
        .with_scl(peripherals.GPIO22);
    let mut i2c = Esp32I2c::new(i2c);
    let bme280_address = detect_bme280_address(&mut i2c);
    let shared_bus = RefCell::new(i2c);

    let sensor = Bme280Sensor::new_with_address(SharedI2cBus::new(&shared_bus), bme280_address);
    let display = Lcd1602Display::new_with_address(
        SharedI2cBus::new(&shared_bus),
        MonotonicDelay,
        LCD1602_ADDRESS_PRIMARY,
    );
    let mut app = ClimateDisplayApp::new_with_config(
        sensor,
        display,
        ClimateDisplayConfig {
            refresh_period_ticks: REFRESH_PERIOD_TICKS,
        },
    );

    println!("original ESP32 climate display started");
    println!(
        "I2C: SDA=GPIO{} SCL=GPIO{} BME280=0x{:02x} LCD1602=0x{:02x}",
        I2C_SDA_GPIO, I2C_SCL_GPIO, bme280_address, LCD1602_ADDRESS_PRIMARY
    );
    println!(
        "refresh: every {} ticks ({} ms loop)",
        REFRESH_PERIOD_TICKS, LOOP_DELAY_MS
    );

    let mut tick = 0u32;
    let mut loop_delay = MonotonicDelay;

    loop {
        match app.tick() {
            Ok(()) => {
                tick += 1;
                if every_nth(tick, REFRESH_PERIOD_TICKS) {
                    println!("climate refresh tick = {}", tick);
                }
            }
            Err(error) => {
                println!("climate tick failed: {:?}", error);
            }
        }

        loop_delay.delay_ms(LOOP_DELAY_MS);
    }
}
