# platform-pc-sim

`platform-pc-sim` は、host PC 上で `hal-api` / `core-app` を検証するための simulator crate です。

## 提供するもの

- `mock_hal`
  - examples / tests / downstream repo から再利用できる mock GPIO / mock I2C
- `climate_sim`
  - `ClimateDisplayApp` を terminal 上で動かすための sensor sequence / 16x2 ASCII renderer
- `virtual_i2c` / `bme280_mock` / `mpu6050_mock`
  - host 上で I2C bus に mock device を attach し、`platform-esp32::Bme280Sensor` や `platform-esp32::Mpu6050Sensor` のような実 driver を board 非依存に検証するための土台
- `hc_sr04_mock`
  - `platform-esp32::HcSr04Sensor` を host 上で検証するための pulse / echo mock device
- `lcd1602_mock` / `dashboard`
  - LCD backpack 書き込みを host 上で可視化し、配線 view / sensor / LCD state / I2C operation をまとめて見る terminal dashboard
- `component_sim` / `web_dashboard`
  - `HC-SR04` / `MPU6050` / servo / dual motor driver の simulator / browser dashboard

## 使いどころ

- 実機が手元にない状態で UI 文言や更新周期を先に詰めたい
- 実機で出るはずの frame を golden test で固定したい
- 新しい board や sensor を追加する前に、共通ロジックの期待挙動を host 上で先に確定したい
- GUI や wiring simulation を作る前に、device-level mock を host 上で育てたい

## 参考コマンド

```bash
cargo run -p platform-pc-sim --bin climate-display-sim
cargo run -p platform-pc-sim --bin climate-dashboard-sim
cargo run -p platform-pc-sim --bin climate-dashboard-sim -- nano
cargo run -p platform-pc-sim --bin device-dashboard-web
cargo run -p platform-pc-sim --bin device-dashboard-web -- nano 7878
cargo test -p platform-pc-sim --all-targets
```
