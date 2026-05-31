# Simulation Bridge — 外部 Rust プロジェクトでの利用ガイド

このドキュメントでは、`mcu-hal-sim-rs` を **外部 Rust プロジェクトの評価・検証基盤** として使う方法を説明します。

---

## 概要

`mcu-hal-sim-rs` の設計は「同じアプリロジックを PC simulator と実機の両方で動かす」ことにあります。

```
あなたのアプリ (core-app の実装)
       │
       ▼
   hal-api traits
       │
  ┌────┴────┐
  │         │
PC simulator  実機 (ESP32 / Pico / ...)
```

外部プロジェクトから使う場合も、この構造は変わりません。

---

## ユースケース

| ユースケース | 方法 |
|------------|------|
| 自作センサーアプリを PC 上で検証したい | `hal-api` を依存に追加し、`platform-pc-sim` のモックで動かす |
| ブラウザでセンサー値をリアルタイム確認したい | `device-dashboard-web` に接続する |
| 既存の firmware ELF を書き込みたい | ダッシュボードの Flash UI を使う |
| 新しいセンサードライバーを追加したい | [porting-and-extension-guide.md](./porting-and-extension-guide.md) を参照 |

---

## 前提条件

- Rust toolchain (stable, 1.66+)
- Cargo workspace
- ESP32 向けには `espflash` (`cargo install espflash`)

```bash
rustup show        # Rust バージョン確認
cargo --version    # Cargo 確認
```

---

## パターン 1: 自分のアプリに `hal-api` を使う

### Cargo.toml への追加

```toml
[dependencies]
hal-api = { git = "https://github.com/1222-takeshi/mcu-hal-sim-rs", default-features = false }
```

`no_std` 環境では `default-features = false` を設定します。  
`std` 環境（PC / test）では既定のままで構いません。

### アプリロジックを `hal-api` trait で書く

```rust
use hal_api::sensor::{EnvSensor, EnvReading};
use hal_api::display::TextDisplay16x2;
use hal_api::error::{SensorError, DisplayError};

pub struct MyApp<S, D> {
    sensor: S,
    display: D,
}

impl<S, D> MyApp<S, D>
where
    S: EnvSensor<Error = SensorError>,
    D: TextDisplay16x2<Error = DisplayError>,
{
    pub fn new(sensor: S, display: D) -> Self {
        Self { sensor, display }
    }

    pub fn tick(&mut self) -> Result<(), ()> {
        let reading = self.sensor.read().map_err(|_| ())?;
        let mut line = heapless::String::<16>::new();
        let _ = core::fmt::write(&mut line, format_args!("{:.1}C", reading.temp_centi_celsius as f32 / 100.0));
        self.display.write_line(0, &line).map_err(|_| ())?;
        Ok(())
    }
}
```

---

## パターン 2: PC Simulator でテスト・動作確認

`platform-pc-sim` のモックを使うことで、実機なしでアプリを検証できます。

### Cargo.toml への追加

```toml
[dev-dependencies]
platform-pc-sim = { git = "https://github.com/1222-takeshi/mcu-hal-sim-rs" }
```

### SequenceEnvSensor でセンサー値を注入

```rust
use platform_pc_sim::climate_sim::{SequenceEnvSensor, TerminalDisplay16x2};
use hal_api::sensor::EnvReading;

fn main() {
    let readings = vec![
        EnvReading { temp_centi_celsius: 2450, humidity_centi_percent: 6000, pressure_pascal: 101325 },
        EnvReading { temp_centi_celsius: 2500, humidity_centi_percent: 5800, pressure_pascal: 101300 },
    ];
    let sensor = SequenceEnvSensor::looping(readings);
    let display = TerminalDisplay16x2::with_stdout();
    let mut app = MyApp::new(sensor, display);

    for _ in 0..10 {
        app.tick().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
```

### VirtualI2cBus でプロトコル単体テスト

I2C ドライバーの動作を検証したいときは `VirtualI2cBus` + `MockBme280Device` を組み合わせます。

```rust
use platform_pc_sim::{
    virtual_i2c::VirtualI2cBus,
    bme280_mock::MockBme280Device,
};
use reference_drivers::bme280::Bme280Sensor;
use hal_api::sensor::EnvSensor;

#[test]
fn test_bme280_reads_temperature() {
    let bus = VirtualI2cBus::default();
    let device = MockBme280Device::new();
    bus.register(0x77, device.clone());

    let mut sensor = Bme280Sensor::new(bus.clone(), 0x77);
    sensor.init().unwrap();

    let reading = sensor.read().unwrap();
    assert!(reading.temp_centi_celsius > 0);
}
```

---

## パターン 3: ブラウザダッシュボードでリアルタイム確認

`device-dashboard-web` を使うと、センサー値・Wiring Diagram・診断パネルをブラウザで確認できます。

### ダッシュボード起動

```bash
# このリポジトリをクローン
git clone https://github.com/1222-takeshi/mcu-hal-sim-rs
cd mcu-hal-sim-rs

# ダッシュボードを起動
cargo run -p platform-pc-sim --bin device-dashboard-web

# ブラウザで開く
open http://localhost:7878
```

### センサー構成の変更

起動時に `board` と `profile` を指定できます。

```bash
# ESP32 + フルセンサー構成
cargo run -p platform-pc-sim --bin device-dashboard-web -- esp32 full

# Nano + 最小構成
cargo run -p platform-pc-sim --bin device-dashboard-web -- nano minimal
```

---

## パターン 4: 外部 Firmware を Flash UI から書き込む

このリポジトリのダッシュボードから、**外部で開発した Rust firmware を直接書き込む**ことができます。

### 前提

- `espflash` がインストールされていること (`cargo install espflash`)
- 対象 firmware が Cargo workspace 形式で、`Cargo.toml` に `[[bin]]` が定義されていること

### 手順

1. ダッシュボードを起動: `cargo run -p platform-pc-sim --bin device-dashboard-web`
2. ブラウザで `http://localhost:7878` を開く
3. **External Firmware** セクションを展開
4. **Firmware directory** に外部プロジェクトのパスを入力:
   ```
   /path/to/your/firmware-project
   ```
5. **Board** を選択 (ESP32 / Pico / Nano)
6. **⚡ Flash** をクリック

ビルドと書き込みが自動で実行され、結果がリアルタイムで表示されます。

### カスタム ELF の直接指定

すでにビルド済みの ELF があれば直接指定することもできます。

```
/path/to/your-project/target/xtensa-esp32-espidf/release/my-firmware
```

---

## 利用可能な HAL trait 一覧

| trait | ファイル | 説明 |
|-------|---------|------|
| `OutputPin` / `InputPin` | `gpio.rs` | GPIO デジタル入出力 |
| `I2cBus` | `i2c.rs` | I2C バス |
| `PwmOutput` | `pwm.rs` | PWM 出力 |
| `EnvSensor` | `sensor.rs` | 温度・湿度・気圧センサー |
| `TextDisplay16x2` | `display.rs` | 16×2 LCD / OLED テキスト表示 |
| `DistanceSensor` | `distance.rs` | 超音波・ToF 距離センサー |
| `ImuSensor` | `imu.rs` | 加速度・ジャイロ (IMU) |
| `LightSensor` | `light.rs` | 照度センサー |
| `GasSensor` | `gas.rs` | ガス・CO2 センサー |
| `RtcDevice` | `rtc.rs` | リアルタイムクロック |
| `ServoMotor` / `DualMotorDriver` | `actuator.rs` | サーボ・DC モーター |
| `CameraCapture` | `camera.rs` | カメラフレーム取得 |

---

## 利用可能なモック一覧 (platform-pc-sim)

| モック | struct | 対応 trait |
|--------|--------|-----------|
| BME280 | `MockBme280Device` | I2C register simulation |
| LCD1602 | `MockLcd1602Device` | `TextDisplay16x2` |
| HC-SR04 | `MockHcSr04Device` | `DistanceSensor` |
| MPU6050 | `MockMpu6050Device` | `ImuSensor` |
| BH1750 | `MockBh1750Device` | `LightSensor` |
| DS3231 | `MockDs3231Device` | `RtcDevice` |
| SGP30 | `MockSgp30Device` | `GasSensor` |
| VL53L0X | `MockVl53l0xDevice` | `DistanceSensor` |
| SSD1306 | `MockSsd1306Device` | `TextDisplay16x2` (OLED) |
| DHT22 | `MockDht22Device` | `EnvSensor` (GPIO) |
| Servo | `MockServoMotor` | `ServoMotor` |
| L298N | `MockDualMotorDriver` | `DualMotorDriver` |
| ESP32-CAM | `MockCamera` | `CameraCapture` |

---

## 参照実装: ClimateDisplayApp

このリポジトリの参照実装である `ClimateDisplayApp` は、同一アプリロジックを 3 経路で動かす例として使えます。

```
platform-pc-sim (mock)  ──┐
platform-esp32 (ESP32)  ──┼──▶ core-app::ClimateDisplayApp
platform-rp2040 (Pico)  ──┘
```

動作確認方法:

```bash
# PC simulator (terminal)
cargo run -p platform-pc-sim --bin climate-display-sim

# PC simulator (ブラウザ dashboard)
cargo run -p platform-pc-sim --bin device-dashboard-web

# ESP32 実機
cd firmware/original-esp32-climate-display && cargo run --release

# Raspberry Pi Pico 実機
cd firmware/raspi-pico-climate-display && cargo run --release
```

---

## 外部プロジェクトのディレクトリ構成例

外部プロジェクトから `hal-api` を使う際の推奨構成です。

```
your-project/
├── Cargo.toml         # workspace
├── crates/
│   ├── app-logic/     # hal-api のみに依存するアプリロジック (no_std)
│   └── platform-impl/ # 実機向け adapter
└── firmware/
    └── your-board/    # [[bin]] エントリーポイント
```

```toml
# your-project/crates/app-logic/Cargo.toml
[dependencies]
hal-api = { git = "https://github.com/1222-takeshi/mcu-hal-sim-rs", default-features = false }
```

テスト時は `platform-pc-sim` を `dev-dependencies` に追加することで、  
実機なしで全ロジックを検証できます。

---

## 関連ドキュメント

- [sensors-and-actuators.md](./sensors-and-actuators.md) — 各センサーの詳細仕様
- [porting-and-extension-guide.md](./porting-and-extension-guide.md) — 新しい board / sensor の追加方法
- [dashboard-guide.md](./dashboard-guide.md) — ブラウザダッシュボードの操作方法
