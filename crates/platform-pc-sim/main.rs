//! # PC Simulator
//!
//! PC上でマイコンアプリケーションをシミュレート実行するバイナリクレート。
//!
//! このシミュレータは、`core-app`のアプリケーションロジックを
//! モックHAL実装（`MockPin`、`MockI2c`）を介して実行します。
//!
//! ## 動作仕様
//!
//! - メインループ: 10ms周期で`App::tick()`を呼び出し
//! - LED出力: コンソールに`[GPIO] Pin XX set HIGH/LOW`と表示
//! - I2C通信: コンソールに`[I2C] Read from 0xXX: N bytes`と表示
//!
//! ## 実行方法
//!
//! ```bash
//! cargo run -p platform-pc-sim
//! ```

use core_app::App;
mod mock_hal;
use mock_hal::{MockI2c, MockPin};
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== PC Simulator Started ===");

    let pin = MockPin::new(13);
    let i2c = MockI2c::new();
    let mut app = App::new(pin, i2c);

    loop {
        if let Err(e) = app.tick() {
            eprintln!("Error: {:?}", e);
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
