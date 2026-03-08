# original-esp32-climate-display

original ESP32 + `BME280` + `LCD1602` I2C backpack 向けの climate display firmware です。

この crate は `core-app::climate_display::ClimateDisplayApp` をそのまま使い、
センサ読み取りと 16x2 表示だけを `platform-esp32` のデバイスドライバへ差し替えます。
目的は、PC simulator で確認した表示ロジックを実機へそのまま持ち込める経路を固定することです。

## いつ使うか

- 表示文言や 16x2 UI を host 上で確認したい:
  - `cargo run -p platform-pc-sim --bin climate-display-sim`
- USB / flash / 汎用 I2C の疎通だけを先に切り分けたい:
  - `firmware/original-esp32-bringup`
- `BME280 + LCD1602` の本命経路を original ESP32 で確認したい:
  - この `original-esp32-climate-display`
- M5StickC を使って Button / onboard I2C を診断したい:
  - `firmware/m5stickc-bringup`

## 対応前提

- ボード: original ESP32 / ESP32-WROOM-32 系
- センサ: `BME280`
- ディスプレイ: `LCD1602` + `PCF8574` 系 I2C backpack
- I2C:
  - `GPIO21` -> `SDA`
  - `GPIO22` -> `SCL`
- 既定アドレス:
  - `BME280`: `0x77` を優先しつつ、firmware 起動時に `0x76` も probe
  - `LCD1602 backpack`: `0x27`

## 実行

```bash
cd firmware/original-esp32-climate-display

# Xtensa toolchain が入っている環境ならそのまま
cargo run --release
```

この firmware は起動後に次を行います。

- BME280 の chip-id / calibration 読み出し
- LCD1602 の 4-bit 初期化
- `ClimateDisplayApp` の tick ループ
- 温度 / 湿度の 16x2 表示更新

シリアルログでは `climate-display heartbeat tick = 100` のような生存確認が出ます。

## この branch での確認範囲

- host 側の `cargo test --workspace --all-targets`
- host 側の `cargo clippy --workspace --all-targets -- -D warnings`
- この crate の `cargo build --release`
- Windows `espflash.exe` 経由の actual flash
- original ESP32 + `BME280` + `LCD1602` で温湿度表示を実機確認
- 実機シリアルで `climate refresh tick = ...` の継続出力を確認

## 既知の前提

- LCD backpack のビット割り当ては、一般的な `0x27` ボードの既定値を前提にしています
- もし文字化けや初期化失敗が出る場合は、`platform-esp32::lcd1602::BackpackMapping` を実機に合わせて変更してください
- この環境では `cargo build --release` は linker 不足で完走しない場合があります。その場合でも `cargo check --release` で型検査までは確認できます
