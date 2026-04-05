#![no_std]
#![no_main]

use core::cell::RefCell;

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};
use embedded_hal::delay::DelayNs;
use esp_backtrace as _;
use hal_api::display::TextFrame16x2;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::{Duration, Instant},
};
use esp_println::println;
use platform_esp32::bme280::{
    BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY, Bme280Config, Bme280Sensor,
};
use platform_esp32::i2c::Esp32I2c;
use platform_esp32::lcd1602::{LCD1602_ADDRESS_PRIMARY, Lcd1602Config, Lcd1602Display};
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

fn should_log_refresh(tick: u32, config: ClimateDisplayConfig) -> bool {
    (config.refresh_on_first_tick && tick == 1)
        || every_nth(tick, config.refresh_period_ticks.max(1))
}

fn frame_line(frame: &TextFrame16x2, row: usize) -> &str {
    core::str::from_utf8(frame.line(row)).unwrap_or("????????????????")
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
    let app_config = ClimateDisplayConfig {
        refresh_period_ticks: REFRESH_PERIOD_TICKS,
        refresh_on_first_tick: true,
    };

    let sensor = Bme280Sensor::new_with_config(
        SharedI2cBus::new(&shared_bus),
        Bme280Config {
            address: bme280_address,
            ..Bme280Config::default()
        },
    );
    let display = Lcd1602Display::new_with_config(
        SharedI2cBus::new(&shared_bus),
        MonotonicDelay,
        Lcd1602Config {
            address: LCD1602_ADDRESS_PRIMARY,
            ..Lcd1602Config::default()
        },
    );
    let mut app = ClimateDisplayApp::new_with_config(sensor, display, app_config);

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
                if should_log_refresh(tick, app_config) {
                    match (app.last_reading(), app.last_frame()) {
                        (Some(reading), Some(frame)) => {
                            println!(
                                "climate refresh tick={} temp_cc={} hum_cp={} line1=\"{}\" line2=\"{}\"",
                                tick,
                                reading.temperature_centi_celsius,
                                reading.humidity_centi_percent,
                                frame_line(&frame, 0),
                                frame_line(&frame, 1)
                            );
                        }
                        _ => println!("climate refresh tick={} frame unavailable", tick),
                    }
                }
            }
            Err(error) => {
                println!("climate tick failed: {:?}", error);
            }
        }

        loop_delay.delay_ms(LOOP_DELAY_MS);
    }
}
