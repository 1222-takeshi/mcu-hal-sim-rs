# platform-rp2040

Raspberry Pi Pico (RP2040) 向けの `hal-api` adapter 層です。

`embedded-hal` v1.0 互換の GPIO / I2C 実装を `hal-api` trait に橋渡しし、
`core-app` を Pico 上で動かすための薄い接続層を提供します。

## 提供する型

| 型 | 説明 |
|---|---|
| `gpio::Rp2040OutputPin<P>` | 出力ピンラッパー |
| `gpio::Rp2040InputPin<P>` | 入力ピンラッパー |
| `i2c::Rp2040I2c<I>` | I2C バスラッパー |

## 使い方

```rust
use rp_pico::hal;
use platform_rp2040::{gpio::Rp2040OutputPin, i2c::Rp2040I2c};
use core_app::App;

// rp2040-hal の型を hal-api adapter でラップ
let pin = Rp2040OutputPin::new(led_pin);
let i2c = Rp2040I2c::new(i2c_bus);
let mut app = App::new(pin, i2c);

loop {
    app.tick().unwrap();
}
```

## ピン配置（Raspberry Pi Pico）

| 機能 | GPIO |
|---|---|
| LED (onboard) | GPIO25 |
| I2C0 SDA | GPIO4 |
| I2C0 SCL | GPIO5 |
| UART0 TX | GPIO0 |
| UART0 RX | GPIO1 |

## 関連クレート

- `firmware/raspi-pico-bringup` — LED / UART / I2C scan + core-app 統合 firmware
