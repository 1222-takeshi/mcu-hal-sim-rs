//! # Custom Timing Example
//!
//! カスタムタイミング制御サンプル。
//!
//! このサンプルは、`App`を使わずに独自のタイミングロジックを実装する方法を示します。
//! 複数の周期的タスクを異なる間隔で実行する例です。
//! モック実装には `platform_pc_sim::mock_hal` を直接使用しています。
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
//! [GPIO] Pin 10 set HIGH
//! [GPIO] Pin 10 set LOW
//! [GPIO] Pin 11 set HIGH
//! [I2C] Read from 0x48: 4 bytes
//! ...
//! ```

use hal_api::gpio::OutputPin;
use hal_api::i2c::I2cBus;
use platform_pc_sim::mock_hal::{MockI2c, MockPin};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Custom Timing Example ===");
    println!("Fast LED (pin 10): 0.5s interval");
    println!("Slow LED (pin 11): 2s interval");
    println!("I2C sensor: 3s interval\n");
    println!("Press Ctrl+C to exit\n");

    // 2つのLEDピン
    let mut fast_led = MockPin::new(10);
    let mut slow_led = MockPin::new(11);

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
