//! # I2C Read Example
//!
//! I2Cセンサ読み取りサンプル。
//!
//! このサンプルは、MockI2cを使用して温度センサ（仮想）からデータを読み取ります。
//! I2Cデバイスとの通信パターンを示しています。
//!
//! ## 実行方法
//!
//! ```bash
//! cargo run --example i2c_read
//! ```
//!
//! ## 期待される出力
//!
//! ```text
//! === I2C Read Example ===
//! Reading from I2C temperature sensor at 0x48...
//! [I2C] Read from 0x48: 4 bytes
//! Temperature data: [255, 255, 255, 255]
//! ...
//! ```

use core_app::App;
use std::thread;
use std::time::Duration;

mod mock_hal {
    use hal_api::error::{GpioError, I2cError};
    use hal_api::gpio::OutputPin;
    use hal_api::i2c::I2cBus;

    pub struct MockPin {
        #[allow(dead_code)]
        pin_number: u8,
        #[allow(dead_code)]
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
            Ok(())
        }

        fn set_low(&mut self) -> Result<(), Self::Error> {
            self.state = false;
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

        fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
            println!("[I2C] Write to 0x{:02X}: {:?}", addr, bytes);
            Ok(())
        }

        fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
            println!("[I2C] Read from 0x{:02X}: {} bytes", addr, buffer.len());
            // 温度センサのダミーデータ（0xFFで埋める）
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
    println!("=== I2C Read Example ===");
    println!("Reading from I2C temperature sensor at 0x48...\n");
    println!("Press Ctrl+C to exit\n");

    // LEDピン（点滅はするが、今回の主役はI2C）
    let pin = MockPin::new(13);

    // 温度センサのI2Cアドレス 0x48（TMP102などで一般的）
    let i2c = MockI2c::new();

    // アプリケーションを初期化
    let mut app = App::new(pin, i2c);

    let mut tick_count = 0;

    // メインループ
    loop {
        if let Err(e) = app.tick() {
            eprintln!("Error: {:?}", e);
            break;
        }

        tick_count += 1;

        // 500 tick（5秒）ごとにセンサデータを表示
        if tick_count % 500 == 0 {
            println!("Sensor read completed (tick: {})\n", tick_count);
        }

        thread::sleep(Duration::from_millis(10));
    }
}
