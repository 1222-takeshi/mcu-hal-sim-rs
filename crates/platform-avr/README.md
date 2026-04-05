# platform-avr

`platform-avr` は、AVR 系 board 向けの generic adapter crate です。

現段階では `arduino-hal` / `avr-hal` の具体型を直接公開せず、`embedded-hal` v1.0 互換の GPIO / I2C を `hal-api` へ橋渡しします。

## 目的

- classic Arduino Nano (`ATmega328P`) を最初の AVR ターゲットとして扱う
- board 固有の初期化は firmware 側に閉じ込める
- `core-app` はそのままに、AVR 系で再利用可能な contract だけを platform 層へ固定する

## 今あるもの

- `gpio::AvrOutputPin`
- `gpio::AvrInputPin`
- `i2c::AvrI2c`
- host 側 integration test による `core-app::App` 接続確認

## 今後

- `arduino-nano-bringup` から実機で使う pin / bus 初期化パターンを抽出する
- 必要になれば `platform-avr` の中に board helper を追加する
- sensor / display driver は、AVR でも共通化価値があるものだけを還流する
