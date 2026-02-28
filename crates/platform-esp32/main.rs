#![cfg_attr(all(feature = "esp32c3", not(feature = "std")), no_std)]
#![cfg_attr(all(feature = "esp32c3", not(feature = "std")), no_main)]

#[cfg(all(feature = "esp32c3", not(feature = "std")))]
mod embedded_main {
    use core_app::App;
    use esp_hal::clock::CpuClock;
    use esp_hal::delay::Delay;
    use esp_hal::gpio::{Level, Output};
    use esp_hal::i2c::master::{Config as I2cConfig, I2c};
    use esp_hal::{entry, Config};

    use crate::esp32_hal::{Esp32I2c, Esp32OutputPin};

    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
        loop {}
    }

    #[entry]
    fn main() -> ! {
        let peripherals = esp_hal::init({
            let mut config = Config::default();
            config.cpu_clock = CpuClock::max();
            config
        });

        let led = Output::new(peripherals.GPIO8, Level::Low);

        let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
            .with_sda(peripherals.GPIO4)
            .with_scl(peripherals.GPIO5);

        let mut app = App::new(Esp32OutputPin::new(led), Esp32I2c::new(i2c));
        let delay = Delay::new();

        loop {
            let _ = app.tick();
            delay.delay_millis(10);
        }
    }
}

#[cfg(not(all(feature = "esp32c3", not(feature = "std"))))]
fn main() {
    println!("platform-esp32 host mode");
    println!("Use one of the following commands for device deployment:");
    println!("  cargo run -p platform-esp32 --no-default-features --features esp32c3 --target riscv32imc-unknown-none-elf");
    println!("  ./scripts/dev-loop.sh flash");
}

mod esp32_hal;
