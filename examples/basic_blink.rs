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
use platform_pc_sim::mock_hal::{MockI2c, MockPin};
use std::thread;
use std::time::Duration;

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
