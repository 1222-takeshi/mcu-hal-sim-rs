# Porting And Extension Guide

このドキュメントは、`mcu-hal-sim-rs` を今後 `Arduino Nano` / `ESP32-CAM` / `Raspberry Pi Pico` / `Teensy` などへ広げるときの設計ガイドです。

---

## 基本方針

- 先に `core-app` をいじらない
- 先に board 固有 firmware で実験する
- 共通化が確認できた契約だけを `hal-api` / `core-app` へ戻す

---

## board / MCU を増やすとき

1. `firmware/<board>-bringup` か別 repo で最小 bring-up を作る
2. 既存 `hal-api` trait で足りるか確認する
3. 足りるなら `platform-<family>` crate を追加して adapter を実装する
4. 足りないなら、board 非依存に説明できる契約だけを `hal-api` へ追加する
5. host simulator で golden test を追加してから実機へ流す

### 追加先の目安

| 変更内容 | 配置場所 |
|---------|---------|
| pin / bus / clock / address の違い | `firmware/*` または `platform-*` |
| board 非依存の sensor/display 契約 | `hal-api` |
| board を跨いで再利用する振る舞い | `core-app` |
| ホスト側モック・シミュレータ | `platform-pc-sim` |
| ESP32 向け具体実装 | `platform-esp32` |
| AVR 向け具体実装 | `platform-avr` |

---

## 新しいセンサーを 4 層追加する手順

新センサーを追加するには、以下の 4 層をこの順序で追加します。

```
hal-api (trait) → reference-drivers (driver) → platform-pc-sim (mock) → platform-esp32 (adapter)
```

### Step 1: `hal-api` に trait を追加

`crates/hal-api/src/` に新しいモジュールを作成します。

```rust
// crates/hal-api/src/my_sensor.rs
use crate::error::SensorError;

/// My sensor reading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MySensorReading {
    pub value: i32,
}

/// Trait for my sensor
pub trait MySensor {
    type Error;

    fn read(&mut self) -> Result<MySensorReading, Self::Error>;
}
```

`crates/hal-api/src/lib.rs` にモジュールを追加します：

```rust
// crates/hal-api/src/lib.rs
pub mod my_sensor;
pub use my_sensor::{MySensor, MySensorReading};
```

### Step 2: `reference-drivers` に driver を実装

`crates/reference-drivers/src/` に driver ファイルを作成します。

```rust
// crates/reference-drivers/src/my_sensor.rs
use hal_api::error::{I2cError, SensorError};
use hal_api::i2c::I2cBus;
use hal_api::my_sensor::{MySensor, MySensorReading};

const MY_SENSOR_REG: u8 = 0x00;

pub struct MySensorDriver<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> MySensorDriver<I2C>
where
    I2C: I2cBus<Error = I2cError>,
{
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }
}

impl<I2C> MySensor for MySensorDriver<I2C>
where
    I2C: I2cBus<Error = I2cError>,
{
    type Error = SensorError;

    fn read(&mut self) -> Result<MySensorReading, Self::Error> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.address, &[MY_SENSOR_REG], &mut buf)
            .map_err(|_| SensorError::ReadFailed)?;
        Ok(MySensorReading {
            value: i16::from_be_bytes(buf) as i32,
        })
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    // テストを追加する（モック I2C を使って read の成功/失敗を検証）
}
```

`crates/reference-drivers/src/lib.rs` に追加：

```rust
pub mod my_sensor;
pub use my_sensor::MySensorDriver;
```

### Step 3: `platform-pc-sim` に mock を追加

`crates/platform-pc-sim/` にモックファイルを作成します。

```rust
// crates/platform-pc-sim/my_sensor_mock.rs
use hal_api::error::SensorError;
use hal_api::my_sensor::{MySensor, MySensorReading};

pub struct MockMySensor {
    value: i32,
    call_count: u32,
}

impl MockMySensor {
    pub fn new(value: i32) -> Self {
        Self { value, call_count: 0 }
    }

    pub fn call_count(&self) -> u32 {
        self.call_count
    }
}

impl MySensor for MockMySensor {
    type Error = SensorError;

    fn read(&mut self) -> Result<MySensorReading, Self::Error> {
        self.call_count += 1;
        Ok(MySensorReading { value: self.value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_my_sensor_returns_configured_value() {
        let mut sensor = MockMySensor::new(42);
        let reading = sensor.read().unwrap();
        assert_eq!(reading.value, 42);
        assert_eq!(sensor.call_count(), 1);
    }
}
```

`crates/platform-pc-sim/lib.rs` に公開：

```rust
pub mod my_sensor_mock;
pub use my_sensor_mock::MockMySensor;
```

### Step 4: `platform-esp32` に adapter を追加

`crates/platform-esp32/` に adapter ファイルを作成します。

```rust
// crates/platform-esp32/my_sensor.rs
//! ESP32 re-export of the board-agnostic MySensor driver.
pub use reference_drivers::my_sensor::*;
```

`crates/platform-esp32/lib.rs` に追加：

```rust
pub mod my_sensor;
```

`crates/platform-esp32/types.rs` に型エイリアスを追加：

```rust
pub type Esp32MySensor = my_sensor::MySensorDriver<SharedI2cBus<'static, Esp32I2c>>;
```

### Step 5: ブリッジテストを追加

`crates/platform-esp32/tests/my_sensor_bridge.rs` を作成：

```rust
use hal_api::my_sensor::MySensor;
use platform_esp32::my_sensor::MySensorDriver;

// ... mock I2C と組み合わせた統合テスト
```

---

## チェックリスト

新センサーを追加したら以下をすべて確認します：

- [ ] `hal-api` に trait + Reading 型が追加されている
- [ ] `reference-drivers` に driver が実装されている（エラーパスのテスト含む）
- [ ] `platform-pc-sim` にモックが追加されている（単体テスト含む）
- [ ] `platform-esp32` に adapter/re-export が追加されている
- [ ] `platform-esp32/types.rs` に型エイリアスが追加されている
- [ ] ブリッジテスト（`platform-esp32/tests/`）が追加されている
- [ ] `docs/sensors-and-actuators.md` に追加されている
- [ ] `cargo test --workspace --all-targets` が全パス
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` が通る
- [ ] `cargo check -p hal-api --lib --target thumbv6m-none-eabi --no-default-features` が通る

---

## sensor を増やすとき

### 既存 `EnvSensor` に載る場合

- 温度 / 湿度 / 気圧など既存 `EnvReading` で表現できるなら、driver は platform crate に追加する
- `ClimateDisplayApp` のような app は既存 trait のまま再利用する

### 既存契約に載らない場合

- IMU / camera / distance / gas sensor のようにデータ形状が違うものは、いきなり `EnvSensor` を拡張しない
- まず新しい trait を `hal-api` に追加するか、board 固有 firmware 側で閉じ込めるかを判断する
- `ESP32-CAM` の camera は board 固有性が強いので、まずは別 firmware / 別 repo での検証を優先する

### 現在追加済みの board 非依存契約

| trait | デバイス例 | モジュール |
|-------|----------|---------|
| `EnvSensor` | BME280, DHT22 | `hal-api::sensor` |
| `DistanceSensor` | HC-SR04, VL53L0X | `hal-api::distance` |
| `ImuSensor` | MPU6050 | `hal-api::imu` |
| `LightSensor` | BH1750 | `hal-api::sensor` |
| `GasSensor` | SGP30 | `hal-api::gas` |
| `RtcSensor` | DS3231 | `hal-api::rtc` |
| `TextDisplay16x2` | LCD1602 | `hal-api::display` |
| `PixelDisplay` | SSD1306 | `hal-api::display` |
| `CameraCapture` | ESP32-CAM | `hal-api::camera` |
| `ServoMotor` | SG90 | `hal-api::actuator` |
| `DualMotorDriver` | L298N | `hal-api::actuator` |

新しい device を増やすときは、まず host-side simulator と GUI に載せてから、
board 固有 adapter や firmware へ流す順序を推奨します。

---

## sim-to-real で維持する契約

- host 側で再現可能な reference input sequence があること
- expected frame / expected telemetry を golden 化できること
- 実機側で同じ値を serial log から照合できること

---

## 今の reference path

```
platform-pc-sim (ClimateDisplaySim)
    ↓
reference-drivers::{Bme280Sensor, Lcd1602Display}
    ↓
core-app::climate_display::ClimateDisplayApp
    ↓
platform-esp32::{Bme280Sensor, Lcd1602Display}
    ↓
firmware/original-esp32-climate-display
```

新しい platform は、この reference path を壊さずに並列追加する方針を取ります。

---

## 実機フラッシュ（ESP32）

```bash
# ワークスペースルートから
./scripts/flash-esp32.sh original-esp32-climate-display   # ポート自動検出
./scripts/flash-esp32.sh original-esp32-robot-base /dev/cu.usbserial-0001

# macOS: /dev/cu.* を自動検出
# Linux: /dev/ttyUSB* を自動検出
# WSL2: ESP32_PORT=COM3 ./scripts/flash-esp32.sh ...
```
