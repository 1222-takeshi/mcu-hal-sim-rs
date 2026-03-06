# platform-esp32

`platform-esp32` は、original Xtensa-based ESP32 向けアダプタ層です。

現段階では `esp-hal` へ直接依存せず、`embedded-hal` v1.0 互換の GPIO / I2C 実装を
`hal-api` に橋渡しするところまでを責務にしています。これにより、`core-app` 側を
変えずに `esp-hal` の具体型を後から接続できます。

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

## 今のスコープ

- `Esp32OutputPin<P>`: `embedded_hal::digital::OutputPin` を `hal_api::gpio::OutputPin` に接続
- `Esp32InputPin<P>`: `embedded_hal::digital::InputPin` を `hal_api::gpio::InputPin` に接続
- `Esp32I2c<I>`: `embedded_hal::i2c::I2c<SevenBitAddress>` を `hal_api::i2c::I2cBus` に接続

## 次にやること

- `esp-hal` の original ESP32 向け GPIO 型を使った薄い生成ヘルパーを追加する
- I2C 初期化の board-specific な引数設計を決める
- 実機で LED 点滅と単純な I2C 読み取りを確認する
