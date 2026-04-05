#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use panic_halt as _;
use ufmt::uWrite;

const SERIAL_BAUD: u32 = 57_600;
const I2C_FREQUENCY_HZ: u32 = 50_000;
const HEARTBEAT_DELAY_MS: u16 = 250;

fn scan_i2c_bus<W>(serial: &mut W, i2c: &mut arduino_hal::I2c)
where
    W: uWrite + ?Sized,
{
    ufmt::uwriteln!(serial, "Write direction test:\r").unwrap_infallible();
    i2c.i2cdetect(serial, arduino_hal::i2c::Direction::Write)
        .unwrap_infallible();
    ufmt::uwriteln!(serial, "\r\nRead direction test:\r").unwrap_infallible();
    i2c.i2cdetect(serial, arduino_hal::i2c::Direction::Read)
        .unwrap_infallible();
    ufmt::uwriteln!(serial, "\r").unwrap_infallible();
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, SERIAL_BAUD);
    let mut led = pins.d13.into_output();
    let mut i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        I2C_FREQUENCY_HZ,
    );

    ufmt::uwriteln!(serial, "arduino nano bring-up started\r").unwrap_infallible();
    ufmt::uwriteln!(
        serial,
        "LED=D13 SDA=A4 SCL=A5 baud={} i2c={}Hz\r",
        SERIAL_BAUD,
        I2C_FREQUENCY_HZ
    )
    .unwrap_infallible();
    ufmt::uwriteln!(
        serial,
        "Use this firmware to confirm blink/serial/I2C before adding sensors.\r"
    )
    .unwrap_infallible();

    scan_i2c_bus(&mut serial, &mut i2c);

    let mut heartbeat = 0u32;
    loop {
        heartbeat = heartbeat.wrapping_add(1);
        led.toggle();

        if heartbeat % 4 == 0 {
            ufmt::uwriteln!(serial, "heartbeat={}\r", heartbeat).unwrap_infallible();
        }

        if heartbeat % 40 == 0 {
            ufmt::uwriteln!(serial, "rescanning I2C bus...\r").unwrap_infallible();
            scan_i2c_bus(&mut serial, &mut i2c);
        }

        arduino_hal::delay_ms(HEARTBEAT_DELAY_MS);
    }
}
