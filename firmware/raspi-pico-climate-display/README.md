# raspi-pico-climate-display

Raspberry Pi Pico (RP2040) + `BME280` + `LCD1602` I2C backpack 向けの climate display firmware です。

この crate は `core-app::climate_display::ClimateDisplayApp` をそのまま使い、
センサ読み取りと 16x2 表示だけを `platform-rp2040` のデバイスドライバへ差し替えます。
目的は、PC simulator で確認した表示ロジックを Pico 実機へそのまま持ち込める経路を固定することです。

## いつ使うか

- 表示文言や 16x2 UI を host 上で確認したい:
  - `cargo run -p platform-pc-sim --bin climate-display-sim`
- USB / UART / 汎用 I2C の疎通だけを先に切り分けたい:
  - `firmware/raspi-pico-bringup`
- `BME280 + LCD1602` の本命経路を Pico で確認したい:
  - この `raspi-pico-climate-display`

## 対応前提

- ボード: Raspberry Pi Pico (RP2040)
- センサ: `BME280`
- ディスプレイ: `LCD1602` + `PCF8574` 系 I2C backpack
- I2C:
  - `GPIO4` -> `SDA`
  - `GPIO5` -> `SCL`
- UART (シリアルログ):
  - `GPIO0` -> `TX`
  - `GPIO1` -> `RX`
  - ボーレート: 115200
- 既定アドレス:
  - `BME280`: `0x77` を優先しつつ、firmware 起動時に `0x76` も probe
  - `LCD1602 backpack`: `0x27`

## 実行

```bash
# firmware ディレクトリ内で実行
cd firmware/raspi-pico-climate-display

# uf2 書き込み (BOOTSEL ボタンを押しながら USB 接続してから)
cargo run --release
```

`elf2uf2-rs` がインストールされていない場合:

```bash
cargo install elf2uf2-rs
```

この firmware は起動後に次を行います。

- BME280 の chip-id probe (`0x77` → `0x76` の順)
- BME280 の calibration 読み出し
- LCD1602 の 4-bit 初期化
- `ClimateDisplayApp` の tick ループ (100 ms / tick)
- 温度 / 湿度の 16x2 表示更新

シリアルログでは次のような refresh telemetry が出ます。

```text
Raspberry Pi Pico climate display started
I2C: SDA=GPIO4 SCL=GPIO5 BME280=0x77 LCD1602=0x27
refresh: every 10 ticks (100 ms loop)
BME280 probe: detected at 0x77 (chip-id=0x60)
climate refresh tick=1 temp_cc=2481 hum_cp=4315 line1="Temp    24.8C   " line2="Hum     43.2%   "
climate refresh tick=10 temp_cc=2483 hum_cp=4312 line1="Temp    24.8C   " line2="Hum     43.1%   "
```

この出力は `original-esp32-climate-display` や simulator 側の expected frame と比較しやすいようにしています。

## 確認範囲

- host 側の `cargo test --workspace --all-targets`
- host 側の `cargo clippy --workspace --all-targets -- -D warnings`
- この crate の `cargo check --release` または toolchain / linker / `elf2uf2-rs` が揃った環境での `cargo build --release`
- Pico + `BME280` + `LCD1602` で温湿度表示を実機確認
- 実機シリアルで `climate refresh tick=...` の継続出力を確認

## 既知の前提

- LCD backpack のビット割り当ては、一般的な `0x27` ボードの既定値を前提にしています
- もし文字化けや初期化失敗が出る場合は、`platform-rp2040::lcd1602::Lcd1602Config` の `mapping` を実機に合わせて変更してください
- `Delay` は SysTick ベースで LCD 初期化と tick ループに共有しています。SysTick を他の用途に使う場合は `SharedDelay` パターンを調整してください
