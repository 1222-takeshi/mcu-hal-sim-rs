# Porting And Extension Guide

このドキュメントは、`mcu-hal-sim-rs` を今後 `Arduino Nano` / `ESP32-CAM` / `Raspberry Pi Pico` / `Teensy` などへ広げるときの設計ガイドです。

## 基本方針

- 先に `core-app` をいじらない
- 先に board 固有 firmware で実験する
- 共通化が確認できた契約だけを `hal-api` / `core-app` へ戻す

## board / MCU を増やすとき

1. `firmware/<board>-bringup` か別 repo で最小 bring-up を作る
2. 既存 `hal-api` trait で足りるか確認する
3. 足りるなら `platform-<family>` crate を追加して adapter を実装する
4. 足りないなら、board 非依存に説明できる契約だけを `hal-api` へ追加する
5. host simulator で golden test を追加してから実機へ流す

### 追加先の目安

- pin / bus / clock / address の違い:
  - `firmware/*` または `platform-*`
- board 非依存の sensor/display 契約:
  - `hal-api`
- board を跨いで再利用する振る舞い:
  - `core-app`

## sensor を増やすとき

### 既存 `EnvSensor` に載る場合

- 温度 / 湿度 / 気圧など既存 `EnvReading` で表現できるなら、driver は platform crate に追加する
- `ClimateDisplayApp` のような app は既存 trait のまま再利用する

### 既存契約に載らない場合

- IMU / camera / distance / gas sensor のようにデータ形状が違うものは、いきなり `EnvSensor` を拡張しない
- まず新しい trait を `hal-api` に追加するか、board 固有 firmware 側で閉じ込めるかを判断する
- `ESP32-CAM` の camera は board 固有性が強いので、まずは別 firmware / 別 repo での検証を優先する

### 現在追加済みの board 非依存契約

- `hal-api::distance::DistanceSensor`
  - `HC-SR04` のような距離センサ向け
- `hal-api::imu::ImuSensor`
  - `MPU6050` のような IMU 向け
- `hal-api::actuator::{ServoMotor, DriveMotor, DualMotorDriver}`
  - servo / DC motor / motor driver 向け

新しい device を増やすときは、まず host-side simulator と GUI に載せてから、
board 固有 adapter や firmware へ流す順序を推奨します。

## sim-to-real で維持する契約

- host 側で再現可能な reference input sequence があること
- expected frame / expected telemetry を golden 化できること
- 実機側で同じ値を serial log から照合できること

## 今の reference path

- `platform-pc-sim`
- `reference-drivers::{Bme280Sensor, Lcd1602Display}`
- `core-app::climate_display::ClimateDisplayApp`
- `platform-esp32::{Bme280Sensor, Lcd1602Display}`
- `firmware/original-esp32-climate-display`

新しい platform は、この reference path を壊さずに並列追加する方針を取ります。
