#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use core::cell::RefCell;
use panic_halt as _;
use platform_avr::bme280::{
    BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY, Bme280Config, Bme280Sensor,
};
use platform_avr::i2c::AvrI2c;
use platform_avr::lcd1602::{LCD1602_ADDRESS_PRIMARY, Lcd1602Config, Lcd1602Display};
use platform_avr::shared_i2c::SharedI2cBus;

use core_app::climate_display::{ClimateDisplayApp, ClimateDisplayConfig};

const SERIAL_BAUD: u32 = 57_600;
const I2C_FREQUENCY_HZ: u32 = 100_000;
const LOOP_DELAY_MS: u32 = 100;
const REFRESH_PERIOD_TICKS: u32 = 10;
const BME280_CHIP_ID_REGISTER: u8 = 0xD0;
const BME280_CHIP_ID_VALUE: u8 = 0x60;

/// Noop delay implementation for the AVR target.
///
/// `arduino_hal::delay_ms` is a free function, not a type, so we wrap it
/// in a zero-size struct that implements `embedded_hal::delay::DelayNs`.
struct AvrDelay;

impl embedded_hal::delay::DelayNs for AvrDelay {
    fn delay_ns(&mut self, ns: u32) {
        // Arduino Nano runs at 16 MHz; minimum useful unit is 1 µs.
        let us = ns.div_ceil(1_000);
        if us > 0 {
            arduino_hal::delay_us(us);
        }
    }

    fn delay_us(&mut self, us: u32) {
        arduino_hal::delay_us(us);
    }

    fn delay_ms(&mut self, ms: u32) {
        arduino_hal::delay_ms(ms);
    }
}

fn detect_bme280_address<B, W>(i2c_bus: &mut B, uart: &mut W) -> u8
where
    B: hal_api::i2c::I2cBus<Error = hal_api::error::I2cError>,
    W: ufmt::uWrite,
{
    for address in [BME280_ADDRESS_PRIMARY, BME280_ADDRESS_SECONDARY] {
        let mut chip_id = [0u8; 1];
        match i2c_bus.write_read(address, &[BME280_CHIP_ID_REGISTER], &mut chip_id) {
            Ok(()) if chip_id[0] == BME280_CHIP_ID_VALUE => {
                let _ = ufmt::uwriteln!(
                    uart,
                    "BME280 probe: detected at 0x{:02x} (chip-id=0x{:02x})\r",
                    address,
                    chip_id[0]
                );
                return address;
            }
            Ok(()) => {
                let _ = ufmt::uwriteln!(
                    uart,
                    "BME280 probe: 0x{:02x} unexpected chip-id=0x{:02x}\r",
                    address,
                    chip_id[0]
                );
            }
            Err(_) => {
                let _ = ufmt::uwriteln!(uart, "BME280 probe: 0x{:02x} failed\r", address);
            }
        }
    }
    let _ = ufmt::uwriteln!(
        uart,
        "BME280 probe: falling back to 0x{:02x}\r",
        BME280_ADDRESS_PRIMARY
    );
    BME280_ADDRESS_PRIMARY
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, SERIAL_BAUD);

    // I2C: SDA=A4, SCL=A5 (Arduino Nano standard pins)
    let raw_i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        I2C_FREQUENCY_HZ,
    );

    let mut avr_i2c = AvrI2c::new(raw_i2c);
    let bme280_address = detect_bme280_address(&mut avr_i2c, &mut serial);

    // Wrap in RefCell for shared bus access.
    let shared_bus = RefCell::new(avr_i2c);

    let sensor = Bme280Sensor::new_with_config(
        SharedI2cBus::new(&shared_bus),
        Bme280Config {
            address: bme280_address,
            ..Bme280Config::default()
        },
    );
    let display = Lcd1602Display::new_with_config(
        SharedI2cBus::new(&shared_bus),
        AvrDelay,
        Lcd1602Config {
            address: LCD1602_ADDRESS_PRIMARY,
            ..Lcd1602Config::default()
        },
    );

    let mut app = ClimateDisplayApp::new_with_config(
        sensor,
        display,
        ClimateDisplayConfig {
            refresh_period_ticks: REFRESH_PERIOD_TICKS,
            refresh_on_first_tick: true,
        },
    );

    ufmt::uwriteln!(serial, "Arduino Nano climate display started\r").unwrap_infallible();
    ufmt::uwriteln!(
        serial,
        "I2C: SDA=A4 SCL=A5 BME280=0x{:02x} LCD1602=0x{:02x}\r",
        bme280_address,
        LCD1602_ADDRESS_PRIMARY
    )
    .unwrap_infallible();

    let mut tick = 0u32;
    loop {
        match app.tick() {
            Ok(()) => {
                tick = tick.wrapping_add(1);
                if tick % REFRESH_PERIOD_TICKS == 0 {
                    ufmt::uwriteln!(serial, "climate tick={}\r", tick).unwrap_infallible();
                }
            }
            Err(_) => {
                ufmt::uwriteln!(serial, "app.tick() error at tick={}\r", tick).unwrap_infallible();
            }
        }

        arduino_hal::delay_ms(LOOP_DELAY_MS);
    }
}
