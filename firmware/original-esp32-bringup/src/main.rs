#![no_std]
#![no_main]

use core_app::App;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Level, Output, OutputConfig},
    main,
};
use esp_println::println;
use hal_api::error::I2cError;
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

#[cfg(feature = "real-i2c")]
struct LoggingI2c<I> {
    inner: I,
}

#[cfg(feature = "real-i2c")]
impl<I> LoggingI2c<I> {
    fn new(inner: I) -> Self {
        Self { inner }
    }
}

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

#[cfg(feature = "real-i2c")]
impl<I> I2cBus for LoggingI2c<I>
where
    I: I2cBus<Error = I2cError>,
{
    type Error = I2cError;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let result = self.inner.write(addr, bytes);
        match &result {
            Ok(()) => println!("i2c write ok: addr=0x{:02x} len={}", addr, bytes.len()),
            Err(error) => println!("i2c write err: addr=0x{:02x} err={:?}", addr, error),
        }
        result
    }

    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let result = self.inner.read(addr, buffer);
        match &result {
            Ok(()) => println!("i2c read ok: addr=0x{:02x} data={:02x?}", addr, buffer),
            Err(error) => println!("i2c read err: addr=0x{:02x} err={:?}", addr, error),
        }
        result
    }

    fn write_read(&mut self, addr: u8, bytes: &[u8], buffer: &mut [u8]) -> Result<(), Self::Error> {
        let result = self.inner.write_read(addr, bytes, buffer);
        match &result {
            Ok(()) => println!(
                "i2c write_read ok: addr=0x{:02x} tx_len={} rx={:02x?}",
                addr,
                bytes.len(),
                buffer
            ),
            Err(error) => println!("i2c write_read err: addr=0x{:02x} err={:?}", addr, error),
        }
        result
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
        LoggingI2c::new(Esp32I2c::new(bus))
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

    let mut loop_count = 0u32;

    loop {
        if let Err(error) = app.tick() {
            println!("tick failed: {:?}", error);
        }
        loop_count += 1;

        if loop_count.is_multiple_of(100) {
            println!("heartbeat tick = {}", loop_count);
        }

        busy_wait(TICK_DELAY_SPINS);
    }
}
