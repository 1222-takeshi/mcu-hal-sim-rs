//! # Basic Blink Example
//!
//! 最小限のLED点滅サンプル。
//!
//! このサンプルは、MockPinを使用してLEDを1秒ごとに点滅させます。
//! `core_app::App`の基本的な使い方を示しています。
//!
//! ## 実行方法
//!
//! ```bash
//! cargo run --example basic_blink
//! ```
//!
//! ## 期待される出力
//!
//! ```text
//! === Basic Blink Example ===
//! [GPIO] Pin 13 set HIGH
//! [GPIO] Pin 13 set LOW
//! [GPIO] Pin 13 set HIGH
//! ...
//! ```

use core_app::App;
use std::thread;
use std::time::Duration;

// platform-pc-simのモックHALを使用
mod mock_hal {
    use hal_api::error::{GpioError, I2cError};
    use hal_api::gpio::OutputPin;
    use hal_api::i2c::I2cBus;

    pub struct MockPin {
        pin_number: u8,
        state: bool,
    }

    impl MockPin {
        pub fn new(pin_number: u8) -> Self {
            Self {
                pin_number,
                state: false,
            }
        }
    }

    impl OutputPin for MockPin {
        type Error = GpioError;

        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.state = true;
            println!("[GPIO] Pin {} set HIGH", self.pin_number);
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.state = false;
            println!("[GPIO] Pin {} set LOW", self.pin_number);
            Ok(())
        }
    }

    pub struct MockI2c;

    impl MockI2c {
        pub fn new() -> Self {
            Self
        }
    }

    impl I2cBus for MockI2c {
        type Error = I2cError;

        fn write(&mut self, _addr: u8, _bytes: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn read(&mut self, _addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
            buffer.fill(0xFF);
            Ok(())
        }

        fn write_read(
            &mut self,
            addr: u8,
            bytes: &[u8],
            buffer: &mut [u8],
        ) -> Result<(), Self::Error> {
            self.write(addr, bytes)?;
            self.read(addr, buffer)
        }
    }
}

use mock_hal::{MockI2c, MockPin};

fn main() {
    println!("=== Basic Blink Example ===");
    println!("Press Ctrl+C to exit\n");

    // GPIO Pin 13を使用（Arduino UnoのオンボードLEDに相当）
    let pin = MockPin::new(13);

    // I2Cは使用しないが、Appの初期化に必要
    let i2c = MockI2c::new();

    // アプリケーションを初期化
    let mut app = App::new(pin, i2c);

    // メインループ: 10msごとにtick()を呼び出す
    // 100 tick = 1秒でLEDが点滅
    loop {
        if let Err(e) = app.tick() {
            eprintln!("Error: {:?}", e);
            break;
        }

        // 10ms待機（100 tick = 1秒）
        thread::sleep(Duration::from_millis(10));
    }
}
