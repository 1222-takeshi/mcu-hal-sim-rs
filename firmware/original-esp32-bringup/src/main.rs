#![no_std]
#![no_main]

use core_app::App;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    main,
};
use esp_println::println;
#[cfg(not(feature = "real-i2c"))]
use hal_api::error::I2cError;
#[cfg(not(feature = "real-i2c"))]
use hal_api::i2c::I2cBus;
use platform_esp32::gpio::Esp32OutputPin;

#[cfg(feature = "real-i2c")]
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
#[cfg(feature = "real-i2c")]
use platform_esp32::i2c::Esp32I2c;

esp_bootloader_esp_idf::esp_app_desc!();

const LED_GPIO: u8 = 2;
const TICK_DELAY_SPINS: u32 = 120_000;
#[cfg(feature = "real-i2c")]
const I2C_SDA_GPIO: u8 = 21;
#[cfg(feature = "real-i2c")]
const I2C_SCL_GPIO: u8 = 22;
const APP_I2C_ADDRESS: u8 = 0x48;

#[cfg(not(feature = "real-i2c"))]
struct NoopI2c;

// `esp_hal::delay` is currently unstable for this stable-toolchain bring-up flow,
// so keep the loop timing coarse and dependency-free.
fn busy_wait(iterations: u32) {
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

#[cfg(not(feature = "real-i2c"))]
impl I2cBus for NoopI2c {
    type Error = I2cError;

    fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    fn read(&mut self, _addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        buffer.fill(0);
        Ok(())
    }

    fn write_read(
        &mut self,
        _addr: u8,
        _bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        buffer.fill(0);
        Ok(())
    }
}

#[main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // `esp-hal` exposes GPIOs as named fields, so keep this in sync with `LED_GPIO`.
    let led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let led = Esp32OutputPin::new(led);

    #[cfg(feature = "real-i2c")]
    let i2c = {
        let bus = I2c::new(peripherals.I2C0, I2cConfig::default())
            .unwrap()
            .with_sda(peripherals.GPIO21)
            .with_scl(peripherals.GPIO22);
        Esp32I2c::new(bus)
    };

    #[cfg(not(feature = "real-i2c"))]
    let i2c = NoopI2c;

    let mut app = App::new(led, i2c);

    println!("original ESP32 bring-up started");
    println!("LED GPIO = {}", LED_GPIO);

    #[cfg(feature = "real-i2c")]
    println!(
        "I2C enabled: SDA = GPIO{}, SCL = GPIO{}, device addr = 0x{:02x}",
        I2C_SDA_GPIO,
        I2C_SCL_GPIO,
        APP_I2C_ADDRESS
    );

    #[cfg(not(feature = "real-i2c"))]
    println!(
        "I2C disabled: build with --features real-i2c when a 0x{:02x} device is connected",
        APP_I2C_ADDRESS
    );

    loop {
        if let Err(error) = app.tick() {
            println!("tick failed: {:?}", error);
        }

        busy_wait(TICK_DELAY_SPINS);
    }
}
