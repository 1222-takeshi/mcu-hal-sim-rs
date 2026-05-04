# Sensors & Actuators Reference

このドキュメントは `mcu-hal-sim-rs` に実装済みのセンサー・アクチュエーターを一覧します。
各デバイスは以下の 4 層すべてに対応しています（DHT22 の ESP32 adapter は一部スタブ）。

```
hal-api  →  reference-drivers  →  platform-pc-sim (mock)  →  platform-esp32 (adapter)
```

---

## センサー一覧

### 1. BME280 — 温湿度・気圧センサー

| 項目 | 値 |
|------|-----|
| 通信方式 | I2C |
| I2C アドレス | `0x77`（primary）/ `0x76`（secondary） |
| HAL trait | `EnvSensor` → `EnvReading { temperature_centi_celsius, humidity_centi_percent, pressure_pa }` |
| ダッシュボードパネル | **Climate** |
| sim-to-real | ✅ 完全対応 |

```rust
use platform_esp32::bme280::{Bme280Sensor, BME280_ADDRESS_PRIMARY};

let mut sensor = Bme280Sensor::new(i2c_bus);
sensor.initialize()?;
let reading = sensor.read()?;
println!("Temp: {:.1}°C", reading.temperature_centi_celsius as f32 / 100.0);
```

---

### 2. MPU6050 — 6軸 IMU（加速度・ジャイロ）

| 項目 | 値 |
|------|-----|
| 通信方式 | I2C |
| I2C アドレス | `0x68`（primary）/ `0x69`（secondary） |
| HAL trait | `ImuSensor` → `ImuReading { accel_mg[3], gyro_mdps[3], temperature_c }` |
| ダッシュボードパネル | **IMU** |
| sim-to-real | ✅ 完全対応 |

```rust
use platform_esp32::mpu6050::{Mpu6050Sensor, MPU6050_ADDRESS_PRIMARY};

let mut sensor = Mpu6050Sensor::new(i2c_bus);
sensor.initialize()?;
let reading = sensor.read()?;
println!("AccelZ: {} mg", reading.accel_mg[2]);
```

---

### 3. HC-SR04 — 超音波距離センサー

| 項目 | 値 |
|------|-----|
| 通信方式 | GPIO（Trig / Echo 各 1 ピン） |
| HAL trait | `DistanceSensor` → `DistanceReading { distance_mm }` |
| 有効レンジ | 20 mm 〜 4000 mm |
| ダッシュボードパネル | **Distance** |
| sim-to-real | ✅ 完全対応 |

```rust
use platform_esp32::hc_sr04::HcSr04Sensor;

let mut sensor = HcSr04Sensor::new(trig_pin, echo_pin, delay);
let reading = sensor.read()?;
println!("Distance: {} mm", reading.distance_mm);
```

---

### 4. BH1750 — 照度センサー ✨ New（PR #70-71）

| 項目 | 値 |
|------|-----|
| 通信方式 | I2C |
| I2C アドレス | `0x23`（ADDR=LOW）/ `0x5C`（ADDR=HIGH） |
| HAL trait | `LightSensor` → `LightReading { lux_x100: u32 }` |
| 計算式 | `lux = raw / 1.2`（精度 1 lx、最大 65535 lx） |
| ダッシュボードパネル | **Light** |
| sim-to-real | ✅ 完全対応 |

```rust
use platform_esp32::bh1750::{Bh1750Sensor, BH1750_ADDRESS_LOW};
use hal_api::light::LightSensor;

let mut sensor = Bh1750Sensor::new(i2c_bus, BH1750_ADDRESS_LOW)?;
let reading = sensor.read_lux()?;
println!("Illuminance: {:.2} lx", reading.lux_x100 as f32 / 100.0);
```

**LightReading の補助メソッド:**

```rust
reading.lux_integer()      // 整数部（lux_x100 / 100）
reading.lux_fractional()   // 小数部（lux_x100 % 100）
```

---

### 5. DHT22 — 温湿度センサー（GPIO 単線）✨ New（PR #70-71）

| 項目 | 値 |
|------|-----|
| 通信方式 | Single-wire GPIO（1 ピン） |
| HAL trait | `EnvSensor` → `EnvReading { temperature_centi_celsius, humidity_centi_percent }` |
| プロトコル | 40 ビット（湿度 16bit + 温度 16bit + チェックサム 8bit） |
| ダッシュボードパネル | Climate（BME280 と互換） |
| sim-to-real | ⚠️ ESP32 adapter はスタブ（`Esp32Dht22RawDevice` の `read_raw_bytes` 未実装） |

```rust
use platform_esp32::dht22::{Esp32Dht22Sensor, Esp32Dht22RawDevice};
use hal_api::sensor::EnvSensor;

// Esp32Dht22RawDevice::read_raw_bytes() の実装が必要
let dev = Esp32Dht22RawDevice::new(gpio_pin, delay);
let mut sensor = Esp32Dht22Sensor::new(dev);
let reading = sensor.read()?; // 実装後に利用可能
```

**PC シミュレーターでの使用（`MockDht22EnvSensor`）:**

```rust
use platform_pc_sim::dht22_mock::MockDht22EnvSensor;
use hal_api::sensor::EnvSensor;

let mut sensor = MockDht22EnvSensor::fixed(256, 623); // 25.6°C, 62.3%RH
let reading = sensor.read()?;
```

> **実装ガイド（`Esp32Dht22RawDevice::read_raw_bytes`）:**
> 1. ホスト開始信号: `set_low` → `delay_ms(18)` → `set_high` → `delay_us(40)`
> 2. センサ応答確認: HIGH→LOW 80µs → LOW→HIGH 80µs
> 3. 40 ビット読み取り: LOW 50µs + HIGH（28µs=0 / 70µs=1）× 40
>
> `esp_idf_svc::hal::gpio::PinDriver`（開放コレクタ設定）+
> `esp_idf_svc::hal::delay::FreeRtos` を使ってください。

---

### 6. SSD1306 — 128×64 OLED ディスプレイ ✨ New（PR #70-71）

| 項目 | 値 |
|------|-----|
| 通信方式 | I2C |
| I2C アドレス | `0x3C`（default）/ `0x3D`（alt） |
| HAL trait | `TextDisplay16x2` → 16 文字 × 2 行のテキスト表示 |
| フォント | 5×8 ピクセル（全 95 ASCII グリフ 0x20–0x7E） |
| 解像度 | 128×64 px（内部は 8 ページ × 128 列） |
| sim-to-real | ✅ 完全対応 |

```rust
use platform_esp32::ssd1306::{Ssd1306Display, SSD1306_ADDRESS_DEFAULT};
use hal_api::display::{TextDisplay16x2, TextFrame16x2};

let mut display = Ssd1306Display::new(i2c_bus, SSD1306_ADDRESS_DEFAULT)?;
let frame = TextFrame16x2::from_lines("Hello, World!   ", "SSD1306 OLED    ");
display.render(&frame)?;
```

> **注意:** `TextDisplay16x2` は 16 文字 × 2 行の制約があります。
> 128×64 の残余エリアは将来の拡張用に予約されています。

---

### 7. ESP32-CAM — カメラ（メタデータスタブ）✨ New（PR #70-71）

| 項目 | 値 |
|------|-----|
| 通信方式 | 内蔵（ESP32-S モジュール専用） |
| HAL trait | `CameraCapture` → `FrameMetadata { width, height, format, frame_size_bytes, sequence }` |
| デフォルト解像度 | 320×240（QVGA） |
| PixelFormat | `Jpeg` / `Rgb565` / `Yuv422` |
| ダッシュボードパネル | **Camera** |
| sim-to-real | ⚠️ メタデータのみ（ピクセルバッファなし） |

```rust
use reference_drivers::esp32_cam::Esp32CamSensor;
use hal_api::camera::CameraCapture;

let mut cam = Esp32CamSensor::default_qvga();
let meta = cam.capture()?;
println!("Frame #{}: {}×{}", meta.sequence, meta.width, meta.height);
```

> **設計ノート:** CLAUDE.md の方針に従い、ESP32-CAM のピクセルバッファ処理は
> 別リポジトリで実装し、共通化できる HAL 抽象のみこのリポジトリに置きます。

---

## アクチュエーター一覧

### 8. Servo — PWM サーボモーター

| 項目 | 値 |
|------|-----|
| 通信方式 | PWM（1 ピン） |
| HAL trait | `ServoMotor` → `set_angle_degrees(0–180)` |
| Duty range | 500µs（0°）〜 2500µs（180°）、50Hz |
| ダッシュボードパネル | **Servo** |
| sim-to-real | ✅ 完全対応 |

```rust
use platform_esp32::servo::ServoDriver;
use platform_esp32::pwm::Esp32PwmOutput;
use hal_api::actuator::ServoMotor;

let pwm = Esp32PwmOutput::new(pwm_channel);
let mut servo = ServoDriver::new(pwm);
servo.set_angle_degrees(90)?; // 中央位置
```

---

### 9. L298N — デュアル H ブリッジモータードライバー

| 項目 | 値 |
|------|-----|
| 通信方式 | GPIO × 2（IN1/IN2）+ PWM × 1（ENA） × 各チャンネル |
| HAL trait | `DriveMotor` → `MotorCommand { direction, duty_percent }` |
| チャンネル | Channel-A（左輪）/ Channel-B（右輪） |
| ダッシュボードパネル | **Motor Driver** |
| sim-to-real | ✅ 完全対応 |

```rust
use platform_esp32::l298n::{L298nChannel, L298nDualDriver};
use hal_api::actuator::{DualMotorDriver, MotorCommand, MotorDirection};

let ch_a = L298nChannel::new(in1_pin, in2_pin, ena_pwm);
let ch_b = L298nChannel::new(in3_pin, in4_pin, enb_pwm);
let mut driver = L298nDualDriver::new(ch_a, ch_b);

driver.apply(
    MotorCommand { direction: MotorDirection::Forward, duty_percent: 60 },
    MotorCommand { direction: MotorDirection::Forward, duty_percent: 60 },
)?;
```

---

## LCD1602 — 16 文字 × 2 行 LCD ディスプレイ

| 項目 | 値 |
|------|-----|
| 通信方式 | I2C（PCF8574 バックパック経由） |
| I2C アドレス | `0x27`（primary）/ `0x3F`（secondary） |
| HAL trait | `TextDisplay16x2` |
| ダッシュボードパネル | **Climate**（物理 LCD 欄） |
| sim-to-real | ✅ 完全対応 |

---

## 対応状況マトリクス

| デバイス | hal-api | reference-drivers | pc-sim mock | esp32 adapter | ダッシュボード |
|---------|:-------:|:-----------------:|:-----------:|:-------------:|:------------:|
| BME280 | ✅ | ✅ | ✅ | ✅ | ✅ Climate |
| MPU6050 | ✅ | ✅ | ✅ | ✅ | ✅ IMU |
| HC-SR04 | ✅ | ✅ | ✅ | ✅ | ✅ Distance |
| BH1750 | ✅ | ✅ | ✅ | ✅ | ✅ Light |
| DHT22 | ✅ | ✅ | ✅ | ⚠️ stub | — |
| SSD1306 | ✅ | ✅ | ✅ | ✅ | — |
| ESP32-CAM | ✅ | ✅ | ✅ | — | ✅ Camera |
| Servo | ✅ | ✅ | ✅ | ✅ | ✅ Servo |
| L298N | ✅ | ✅ | ✅ | ✅ | ✅ Motor |
| LCD1602 | ✅ | ✅ | ✅ | ✅ | ✅ Climate |

> ⚠️ stub = コンパイルは通るが `SensorError::NotInitialized` を返す。実 HAL 実装が必要。

---

## ローカル動作確認

```bash
# シミュレーター起動
cargo run -p platform-pc-sim --bin device-dashboard-web
# → http://127.0.0.1:7878 をブラウザで開く

# すべてのテスト
cargo test --workspace --all-targets

# API でセンサー値を確認
curl http://127.0.0.1:7878/api/state | python3 -m json.tool
```

> **ポート競合エラーが出る場合:** 旧プロセスが残っている可能性があります。
> `lsof -ti :7878 | xargs kill` でクリアしてから再起動してください。

---

## 新しいセンサーを追加するには

[porting-and-extension-guide.md](./porting-and-extension-guide.md) を参照してください。
大まかな手順:

1. `hal-api` に trait を追加（`no_std`、doc test 必須）
2. `reference-drivers` に driver を実装（`no_std`）
3. `platform-pc-sim` に mock を追加（VirtualI2cDevice or 直接実装）
4. `device_dashboard_web.rs` と `web_dashboard.rs` にパネルを追加
5. `platform-esp32` に re-export または adapter を追加
6. PR → CI グリーン → AI レビュー → マージ
