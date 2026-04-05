# platform-esp32

`platform-esp32` は、original Xtensa-based ESP32 向けアダプタ層です。

現段階では `esp-hal` へ直接依存せず、`embedded-hal` v1.0 互換の GPIO / I2C 実装を
`hal-api` に橋渡しします。reference path で使う `BME280` / `LCD1602` / `MPU6050` / `HC-SR04` driver 本体は
`reference-drivers` crate へ切り出し、この crate から re-export しています。
これにより、`core-app` 側を変えずに `esp-hal` の具体型を後から接続でき、将来の
`Arduino Nano` / `Raspberry Pi Pico` / `Teensy` でも同じ driver を再利用できます。

エラーは `embedded-hal` の generic error kind から `hal_api::error::{GpioError, I2cError}`
へ正規化しているため、`core-app::App` がそのまま利用できます。

## 想定ターゲット

- チップ: original ESP32
- Rust target: `xtensa-esp32-none-elf`
- フラッシュ: `espflash`

## セットアップ

公式の Rust on ESP / `esp-hal` ドキュメントに従って toolchain を準備してください。

- Rust on ESP Book: <https://docs.espressif.com/projects/rust/book/>
- `esp-hal` for ESP32: <https://docs.espressif.com/projects/rust/esp-hal/latest/esp32/esp_hal/>

1. `espup` で Xtensa 向け Rust toolchain を導入する
2. シェルへ export された環境変数を読み込む
3. `espflash` をインストールする

## 最小確認

```bash
# repo ルートで実行
cargo check-esp32
```

`.cargo/config.toml` で `xtensa-esp32-none-elf` 向け runner と alias を定義しています。

## 実機で確認済み

- original ESP32 + CP210x USB-UART bridge
- `espflash board-info`
- `firmware/original-esp32-bringup` の LED only flash / boot log
- WSL2 host では build を WSL、flash を Windows `espflash.exe` に分ける経路

## ホスト OS 方針

- 実行ホストは native macOS / native Linux / Windows / WSL2 を想定する
- flash 手順を提案するときは、Windows の `COMx` だけを前提にせず、macOS / Linux の serial device path も考慮する
- WSL2 は native serial が見えない場合の例外経路として扱い、native macOS / Linux は通常の `espflash` 実行経路として扱う

## 今のスコープ

- `Esp32OutputPin<P>`: `embedded_hal::digital::OutputPin` を `hal_api::gpio::OutputPin<Error = GpioError>` に接続
- `Esp32InputPin<P>`: `embedded_hal::digital::InputPin` を `hal_api::gpio::InputPin<Error = GpioError>` に接続
- `Esp32I2c<I>`: `embedded_hal::i2c::I2c<SevenBitAddress>` を `hal_api::i2c::I2cBus<Error = I2cError>` に接続
- `SharedI2cBus<'a, B>`: 1本の I2C バスを複数デバイスへ共有
- `Bme280Sensor<B>`: `reference-drivers` から re-export される board 非依存 driver
- `Bme280Config`: address / control register をまとめた config
- `Lcd1602Display<B, D>`: `reference-drivers` から re-export される board 非依存 driver
- `Lcd1602Config`: address / backpack mapping / backlight 初期値をまとめた config
- `Mpu6050Sensor<B>`: `reference-drivers` から re-export される board 非依存 IMU driver
- `Mpu6050Config`: address / power / filter / range 設定をまとめた config
- `HcSr04Sensor<D>`: `reference-drivers` から re-export される board 非依存 distance driver
- `HcSr04Config`: echo 計測の許容レンジをまとめた config
- `tests/app_bridge.rs`: `core-app::App` と `platform-esp32` アダプタの組み合わせを host 上で検証
- `tests/climate_bridge.rs`: `core-app::climate_display::ClimateDisplayApp` が BME280 / LCD1602 と同じ I2C バスで動くことを host 上で検証

## 実機向けの想定経路

- bring-up 用: `firmware/original-esp32-bringup`
- climate display 本体: `firmware/original-esp32-climate-display`
- 補助診断ボード: `firmware/m5stickc-bringup`

後者は `BME280(0x77)` と `LCD1602 backpack(0x27)` を前提に、temperature / humidity を 16x2 表示へ流します。
`m5stickc-bringup` は同じ original ESP32 系でも board 固有部品が多い M5StickC を対象に、Button / onboard I2C の切り分けを短く回すための補助経路です。

## 次にやること

- `esp-hal` の original ESP32 向け GPIO 型を使った薄い生成ヘルパーを追加する
- board / sensor / display ごとの差分を config struct に閉じ込める
- `Arduino Nano` / `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` へ広げる場合に共通化すべき contract を切り出す
- `original-esp32-climate-display` の実機検証を reference path として維持する
