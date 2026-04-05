# reference-drivers

`reference-drivers` は、reference path で使う I2C sensor / display driver を board 非依存にまとめた crate です。

## 提供するもの

- `bme280`
  - `hal_api::sensor::EnvSensor` を実装する `Bme280Sensor`
- `hc_sr04`
  - `hal_api::distance::DistanceSensor` を実装する `HcSr04Sensor`
- `lcd1602`
  - `hal_api::display::TextDisplay16x2` を実装する `Lcd1602Display`
- `mpu6050`
  - `hal_api::imu::ImuSensor` を実装する `Mpu6050Sensor`

## 位置づけ

- `platform-esp32` はこの crate を re-export し、board adapter に集中する
- 将来の `platform-avr` / `platform-rp2040` / `platform-teensy` も同じ driver を使える
- host simulator / GUI / wiring simulation は、この crate と `platform-pc-sim` の mock device を組み合わせて board 名に依存せず検証する

## 参考コマンド

```bash
cargo test -p reference-drivers --all-targets
```
