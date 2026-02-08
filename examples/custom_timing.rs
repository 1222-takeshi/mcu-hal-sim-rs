//! # Custom Timing Example
//!
//! カスタムタイミング制御サンプル。
//!
//! このサンプルは、`App`を使わずに独自のタイミングロジックを実装する方法を示します。
//! 複数の周期的タスクを異なる間隔で実行する例です。
//!
//! ## 実行方法
//!
//! ```bash
//! cargo run --example custom_timing
//! ```
//!
//! ## 期待される出力
//!
//! ```text
//! === Custom Timing Example ===
//! [Fast LED] Pin 10 set HIGH
//! [Fast LED] Pin 10 set LOW
//! [Slow LED] Pin 11 set HIGH
//! [I2C] Read from sensor...
//! ...
//! ```

use std::thread;
use std::time::Duration;

mod mock_hal {
    use hal_api::error::{GpioError, I2cError};
    use hal_api::gpio::OutputPin;
    use hal_api::i2c::I2cBus;

    pub struct MockPin {
        pin_number: u8,
        state: bool,
        label: String,
    }

    impl MockPin {
        pub fn new(pin_number: u8, label: &str) -> Self {
            Self {
                pin_number,
                state: false,
                label: label.to_string(),
            }
        }
    }

    impl OutputPin for MockPin {
        type Error = GpioError;

        fn set_high(&mut self) -> Result<(), Self::Error> {
            self.state = true;
            println!("[{}] Pin {} set HIGH", self.label, self.pin_number);
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.state = false;
            println!("[{}] Pin {} set LOW", self.label, self.pin_number);
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

        fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
            println!("[I2C] Read from sensor 0x{:02X}...", addr);
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

use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;
use mock_hal::{MockI2c, MockPin};

fn main() {
    println!("=== Custom Timing Example ===");
    println!("Fast LED: 0.5s interval");
    println!("Slow LED: 2s interval");
    println!("I2C sensor: 3s interval\n");
    println!("Press Ctrl+C to exit\n");

    // 2つのLEDピン
    let mut fast_led = MockPin::new(10, "Fast LED");
    let mut slow_led = MockPin::new(11, "Slow LED");

    // I2Cセンサ
    let mut i2c = MockI2c::new();

    // カウンタ
    let mut tick_count = 0;
    let mut fast_led_state = false;
    let mut slow_led_state = false;

    // メインループ: 10msごとに実行
    loop {
        tick_count += 1;

        // タスク1: 高速LED点滅（50 tick = 500ms）
        if tick_count % 50 == 0 {
            fast_led_state = !fast_led_state;
            if let Err(e) = fast_led.set(fast_led_state) {
                eprintln!("Fast LED error: {:?}", e);
                break;
            }
        }

        // タスク2: 低速LED点滅（200 tick = 2s）
        if tick_count % 200 == 0 {
            slow_led_state = !slow_led_state;
            if let Err(e) = slow_led.set(slow_led_state) {
                eprintln!("Slow LED error: {:?}", e);
                break;
            }
        }

        // タスク3: センサ読み取り（300 tick = 3s）
        if tick_count % 300 == 0 {
            let mut buffer = [0u8; 4];
            if let Err(e) = i2c.read(0x48, &mut buffer) {
                eprintln!("I2C error: {:?}", e);
                break;
            }
            println!();
        }

        // 10ms待機
        thread::sleep(Duration::from_millis(10));
    }
}
