# arduino-nano-bringup

classic Arduino Nano (`ATmega328P`) 向けの bring-up firmware です。

この crate は、`mcu-hal-sim-rs` が今後 `Arduino Nano` 系まで広がることを見据えた最初の実機足場です。  
現時点では `core-app` との統合より前に、次の 3 点を最短で確認する目的に絞っています。

- onboard LED (`D13`) の点滅
- USB serial の疎通
- onboard I2C (`A4` / `A5`) にぶら下がる sensor の address 検出

## 対象

- board: classic Arduino Nano
- MCU: `ATmega328P`
- LED: `D13`
- I2C:
  - `A4` -> `SDA`
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

- `arduino nano bring-up started`
- `LED=D13 SDA=A4 SCL=A5 ...`
- `Write direction test:` / `Read direction test:` の I2C detect 結果
- `heartbeat=...`

40 heartbeat ごとに I2C bus を再 scan します。  
新しい sensor を足す前に、まずこの firmware で address が見えることを確認してください。

## この crate の位置づけ

- これは `platform-avr` や `core-app` 連携の前段です
- まず board 固有の bring-up を固定し、その後に共通 contract へ還流する方針です
- `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` も同じ考え方で、最初は bring-up firmware から始めます

## 未検証事項

- この環境では AVR toolchain が未導入のため、`cargo run --release` の実機検証は未実施です
- 旧 bootloader / clone board では `ravedude` の board 指定や baud を調整する必要があります
