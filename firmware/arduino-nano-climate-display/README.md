# arduino-nano-climate-display

classic Arduino Nano (`ATmega328P`) + `BME280` + `LCD1602` I2C backpack 向けの climate display firmware です。

この crate は `core-app::climate_display::ClimateDisplayApp` をそのまま使い、
センサ読み取りと 16x2 表示だけを `platform-avr` のデバイスドライバへ差し替えます。
目的は、PC simulator で確認した表示ロジックを Arduino Nano 実機へそのまま持ち込める経路を固定することです。

## いつ使うか

- 表示文言や 16x2 UI を host 上で確認したい:
  - `cargo run -p platform-pc-sim --bin climate-display-sim`
- LED / serial / I2C の疎通だけを先に切り分けたい:
  - `firmware/arduino-nano-bringup`
- `BME280 + LCD1602` の本命経路を Arduino Nano で確認したい:
  - この `arduino-nano-climate-display`

## 対応前提

- ボード: classic Arduino Nano (`ATmega328P`)
- センサ: `BME280`
- ディスプレイ: `LCD1602` + `PCF8574` 系 I2C backpack
- I2C:
  - `A4` -> `SDA`
  - `A5` -> `SCL`
- シリアルログ:
  - ボーレート: 57600
- 既定アドレス:
  - `BME280`: `0x77` を優先しつつ、firmware 起動時に `0x76` も probe
  - `LCD1602 backpack`: `0x27`

## 配線図

```
Arduino Nano          BME280
-----------           ------
A4 (SDA) ----------- SDA
A5 (SCL) ----------- SCL
5V  ---------------- VCC (または 3.3V)
GND ----------------- GND

Arduino Nano          LCD1602 I2C backpack (PCF8574)
-----------           ----------------------------
A4 (SDA) ----------- SDA
A5 (SCL) ----------- SCL
5V  ---------------- VCC
GND ----------------- GND
```

BME280 と LCD1602 backpack はどちらも同じ I2C バスに接続します。
- `BME280`: `0x77` (SDO を HIGH) または `0x76` (SDO を LOW)
- `LCD1602 backpack`: `0x27`

## 必要なもの

`avr-hal` の公式 Quickstart に従い、AVR toolchain を準備してください。

- `avr-hal` README: https://github.com/Rahix/avr-hal
- `avr-hal-template`: https://github.com/Rahix/avr-hal-template

必要なもの:

- nightly Rust (`nightly-2025-04-27` 以降)
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

## ビルド・フラッシュ手順

```bash
# firmware ディレクトリ内で実行
cd firmware/arduino-nano-climate-display

# build + flash + serial monitor (ravedude が Nano に書き込み)
cargo run --release
```

`.cargo/config.toml` では以下を前提にしています。

- target: `avr-none`
- cpu: `atmega328p`
- runner: `ravedude nano -cb 57600`

`ravedude` の board / baud は利用する Nano や bootloader によって調整が必要です。

## この firmware の動作

起動後に次を行います。

- BME280 の chip-id probe (`0x77` → `0x76` の順)
- BME280 の calibration 読み出し
- LCD1602 の 4-bit 初期化
- `ClimateDisplayApp` の tick ループ (250 ms / tick)
- 温度 / 湿度の 16x2 表示更新

## 期待されるシリアル出力

シリアルモニタを 57600 baud で接続すると次のような出力が確認できます。

```text
arduino nano climate display started
I2C: SDA=A4 SCL=A5 BME280=0x77 LCD1602=0x27
refresh: every 10 ticks (250 ms loop)
BME280 probe: detected at 0x77 (chip-id=0x60)
climate refresh tick=1 temp_cc=2481 hum_cp=4315 line1="Temp    24.8C   " line2="Hum     43.2%   "
climate refresh tick=10 temp_cc=2483 hum_cp=4312 line1="Temp    24.8C   " line2="Hum     43.1%   "
```

この出力は `original-esp32-climate-display` や `raspi-pico-climate-display`、
simulator 側の expected frame と比較しやすいようにしています。

## 確認範囲

- host 側の `cargo test --workspace --all-targets`
- host 側の `cargo clippy --workspace --all-targets -- -D warnings`
- この crate の `cargo check --release` または AVR toolchain が揃った環境での `cargo build --release`
- Arduino Nano + `BME280` + `LCD1602` で温湿度表示を実機確認
- 実機シリアル (57600 baud) で `climate refresh tick=...` の継続出力を確認

## トラブルシューティング

### avr-hal は nightly 必須

`rust-toolchain.toml` に `nightly-2025-04-27` を指定しています。
AVR ターゲットは nightly の `build-std` を必要とするため、stable では build できません。

```bash
# toolchain の確認
rustup show
rustup install nightly-2025-04-27
rustup component add rust-src --toolchain nightly-2025-04-27
```

### ravedude が Nano を認識しない

clone 品の Nano では CH340 系 USB-Serial が使われる場合があります。
macOS では `brew install --cask wch-ch34x-usb-serial-driver` でドライバをインストールしてください。

```bash
# 接続確認
ls /dev/cu.* | grep -i usb
# ravedude の port 明示 (例)
ravedude nano -cb 57600 -P /dev/cu.usbserial-1410
```

### baud の不一致

Arduino Nano の旧 bootloader は 57600 baud を使います。
新しい Optiboot ベースの bootloader は 115200 baud を使う場合があります。
`.cargo/config.toml` の `ravedude nano -cb 57600` を実際の bootloader に合わせて変更してください。

### LCD backpack のビット割り当て

LCD backpack のビット割り当ては、一般的な `0x27` ボードの既定値を前提にしています。
もし文字化けや初期化失敗が出る場合は、`platform-avr::lcd1602::Lcd1602Config` の
`mapping` を実機に合わせて変更してください。
