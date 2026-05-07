# arduino-nano-bringup

classic Arduino Nano (`ATmega328P`) 向けの bring-up firmware です。

この crate は `mcu-hal-sim-rs` の sim-to-real 経路を AVR 系 board まで伸ばす最初の実機足場です。  
`platform-avr` の `AvrOutputPin` / `AvrI2c` アダプタを通して `core-app::App` を実行することで、  
PC シミュレータと同じアプリロジックを Arduino Nano 実機上で動作させます。

## 起動シーケンス

1. `scan_i2c_bus` で A4/A5 に繋がる sensor address を検出し serial に出力
2. `arduino_hal` の pin / I2C を `platform-avr` アダプタでラップ
3. `App::new(avr_pin, avr_i2c)` で `core-app` を生成し、`app.tick()` ループへ

## 対象

- board: classic Arduino Nano
- MCU: `ATmega328P`
- LED: `D13` (`AvrOutputPin` でラップ)
- I2C:
  - `A4` -> `SDA` (`AvrI2c` でラップ)
  - `A5` -> `SCL`

## 前提

`avr-hal` の公式 Quickstart に従い、AVR toolchain を準備してください。

- `avr-hal` README: https://github.com/Rahix/avr-hal
- `avr-hal-template`: https://github.com/Rahix/avr-hal-template

必要なもの:

- nightly Rust
- `rust-src`
- `avr-gcc`
- `avrdude`
- `ravedude`

macOS の例:

```bash
xcode-select --install
brew tap osx-cross/avr
brew install avr-gcc avrdude
cargo +stable install ravedude
```

## 実行

```bash
cd firmware/arduino-nano-bringup

# build + flash + serial monitor
cargo run --release
```

`.cargo/config.toml` では以下を前提にしています。

- target: `avr-none`
- cpu: `atmega328p`
- runner: `ravedude nano -cb 57600`

`ravedude` の board / baud は利用する Nano や bootloader によって調整が必要です。

## ログで確認すること

- `arduino nano bring-up + hal-api demo`
- `LED=D13 SDA=A4 SCL=A5 ...`
- `Write direction test:` / `Read direction test:` の I2C detect 結果
- `Starting core-app via hal-api adapters...`
- `heartbeat=...` — `core-app::App::tick()` が 100 tick ごとに LED を制御

## コード構成

```
arduino-nano-bringup/src/main.rs
 ├─ scan_i2c_bus()        : 起動時の I2C bus scan (arduino_hal 直接)
 ├─ AvrOutputPin::new(led): D13 を hal-api OutputPin でラップ
 ├─ AvrI2c::new(i2c)     : TWI を hal-api I2cBus でラップ
 └─ App::new(pin, i2c)   : platform 非依存のアプリロジック実行
```

## この crate の位置づけ

- `platform-avr` (`AvrOutputPin` / `AvrI2c`) と `core-app` の統合を実機で示す
- PC シミュレータ (`platform-pc-sim`) と同じ `App` 型をそのまま再利用
- board 固有の初期化 (`arduino_hal` 呼び出し) はこの firmware に閉じ込め、`platform-avr` を汚染しない

## 未検証事項

- この環境では AVR toolchain が未導入のため、`cargo run --release` の実機検証は未実施
- 旧 bootloader / clone board では `ravedude` の board 指定や baud を調整する必要があります
- `arduino_hal::I2c` の `embedded-hal v1.0` 対応は avr-hal commit `e5c8f37` 以降に依存します

