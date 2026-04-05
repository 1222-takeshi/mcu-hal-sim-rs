# hal-api

`hal-api` は、`mcu-hal-sim-rs` の最下層にある board 非依存の HAL trait crate です。

この crate 自体は driver 実装を持たず、次のような「移植時に変わりやすい境界」だけを定義します。

- GPIO
- I2C
- 環境センサ読み取り
- 距離センサ読み取り
- IMU 読み取り
- サーボ / モータドライバ出力
- 16x2 テキスト表示

## 使いどころ

- PC simulator と実機で同じアプリロジックを再利用したい
- board 固有の HAL を直接 `core-app` に漏らしたくない
- 新しい board / MCU / sensor を追加するときに、共通ロジックの依存面を固定したい

## 現在の前提

- `no_std`
- 同梱している抽象は、現在の reference path である `EnvSensor` / `TextDisplay16x2` を中心にしつつ、
  将来の `HC-SR04` / `MPU6050` / servo / DC motor / dual motor driver を載せるための最小契約も含む
- `DistanceSensor` に加えて `UltrasonicPulseDevice` を持ち、`HC-SR04` のような trigger / echo 型センサを
  board 依存コードから切り離して driver 化できる
- 将来 `Arduino Nano` / `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` 系へ広げる場合も、まずはこの crate に board 非依存の契約だけを追加する想定

## 関連 crate

- `core-app`: `hal-api` の trait だけに依存する共通アプリロジック
- `platform-pc-sim`: host 実行用の mock / simulator
- `platform-esp32`: original ESP32 向け adapter / driver
