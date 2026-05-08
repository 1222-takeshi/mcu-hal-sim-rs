# CLAUDE.md - mcu-hal-sim-rs

このファイルは、`mcu-hal-sim-rs` プロジェクト固有の開発コンテキストを提供します。

## プロジェクト概要

`mcu-hal-sim-rs` は、マイコン向け Rust アプリケーションを **MCU 非依存の HAL trait 経由で記述し、PC simulator で検証してから実機へ持っていくための基盤 repo** です。

現在の reference path は次です。

```text
platform-pc-sim -> core-app -> platform-esp32 -> original ESP32 + BME280 + LCD1602
```

この repo では `hal-api` / `core-app` / `reference-drivers` / `platform-*` の責務分離と、sim-to-real 契約の固定を優先します。`M5StickC` は補助診断ボード、`Arduino Nano` / `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` は将来候補または別 repo 先行の拡張候補として扱います。

## 現在のフェーズ

- ✅ PC simulator / core HAL trait / app logic の基盤
- ✅ CI/CD と workspace test / clippy / format / no_std check
- ✅ `hal-api` と `core-app` の `no_std` 維持
- ✅ `platform-esp32` の adapter 層と original ESP32 bring-up firmware
- ✅ `ClimateDisplayApp` の PC simulator と original ESP32 climate display reference path
- ✅ BME280 / LCD1602 / SharedI2cBus の基本・異常系テスト
- 🚧 Phase 4: no_std 対応と original ESP32 実機確認手順の維持・拡張

## スコープ方針

### 本 repo に残すもの

- `hal-api` の汎用抽象
- `core-app` の再利用可能なアプリロジック
- `reference-drivers` の board 非依存 driver
- `platform-pc-sim` / `platform-avr` / `platform-esp32` の sim-to-real 経路
- original ESP32 + BME280 + LCD1602 の本線シナリオ
- 共通化価値がある sensor / display / actuator 契約とテスト

### 本 repo に先に入れないもの

- 特定 board だけで完結する実験的 UI
- camera / wireless / board 固有周辺機能の寄せ集め
- 個別プロダクト向けアプリ要件そのもの

特に `ESP32-CAM` は camera / framebuffer / board 固有配線を含むため、まず別 repo で進め、必要になった最小抽象だけを本 repo に還流します。

## プロジェクト構成

```text
mcu-hal-sim-rs/
├── crates/
│   ├── hal-api/             # HAL trait / common error types
│   │   ├── actuator.rs      # ServoMotor / DriveMotor / DualMotorDriver
│   │   ├── camera.rs        # CameraCapture / FrameMetadata
│   │   ├── distance.rs      # DistanceSensor
│   │   ├── display.rs       # TextDisplay16x2 / TextFrame16x2
│   │   ├── error.rs         # GPIO / I2C / sensor / display / actuator errors
│   │   ├── gpio.rs          # OutputPin / InputPin
│   │   ├── i2c.rs           # I2cBus
│   │   ├── imu.rs           # ImuSensor
│   │   └── sensor.rs        # EnvSensor / EnvReading
│   │
│   ├── core-app/            # platform 非依存 app logic
│   │   ├── climate_display.rs # ClimateDisplayApp
│   │   ├── imu_logger.rs
│   │   └── lib.rs             # App<PIN, I2C>
│   │
│   ├── reference-drivers/   # board 非依存 reference drivers
│   │   ├── bme280.rs
│   │   ├── lcd1602.rs
│   │   ├── hc_sr04.rs
│   │   ├── mpu6050.rs
│   │   ├── sgp30.rs
│   │   ├── ssd1306.rs
│   │   ├── vl53l0x.rs
│   │   ├── servo.rs
│   │   └── l298n.rs
│   │
│   ├── platform-pc-sim/     # host simulator / mocks / dashboards
│   │   ├── virtual_i2c.rs
│   │   ├── bme280_mock.rs
│   │   ├── lcd1602_mock.rs
│   │   ├── climate_sim.rs
│   │   ├── climate_display_sim.rs
│   │   ├── climate_dashboard_sim.rs
│   │   ├── component_sim.rs
│   │   └── device_dashboard_web.rs
│   │
│   ├── platform-esp32/      # original ESP32 adapter layer
│   │   ├── gpio.rs          # Esp32OutputPin / Esp32InputPin
│   │   ├── i2c.rs           # Esp32I2c
│   │   ├── pwm.rs           # Esp32PwmOutput
│   │   ├── shared_i2c.rs    # SharedI2cBus
│   │   ├── bme280.rs        # reference-drivers re-export
│   │   ├── lcd1602.rs       # reference-drivers re-export
│   │   └── types.rs         # composed type aliases
│   │
│   └── platform-avr/        # classic AVR adapter layer
│       ├── gpio.rs
│       └── i2c.rs
│
├── firmware/
│   ├── original-esp32-bringup/
│   ├── original-esp32-climate-display/
│   ├── m5stickc-bringup/
│   └── arduino-nano-bringup/
├── docs/
├── scripts/
└── .github/workflows/ci.yml
```

## 依存関係の考え方

```text
platform-pc-sim ─┐
platform-esp32 ──┼─> core-app ─> hal-api
platform-avr  ───┘

reference-drivers -> hal-api / embedded-hal
platform-esp32    -> reference-drivers re-export + adapter wrappers
platform-pc-sim   -> host-side mocks and dashboards
```

`core-app` は board 固有型に依存させず、`hal-api` trait と error contract のみを見るようにします。

## よく使うコマンド

```bash
# workspace 全体のテスト
cargo test --workspace --all-targets

# format / lint
cargo fmt --all -- --check
cargo clippy --all --all-targets -- -D warnings

# ローカル CI 相当
./scripts/ci-local.sh

# no_std / ESP32 関連チェック
cargo check -p hal-api --lib --target thumbv6m-none-eabi
cargo check -p core-app --lib --target thumbv6m-none-eabi
cargo check -p platform-esp32 --lib --target thumbv6m-none-eabi
cargo check-esp32

# reference path 周辺の絞り込み
cargo test -p core-app climate_display
cargo test -p reference-drivers bme280
cargo test -p reference-drivers lcd1602
cargo test -p platform-esp32 shared_i2c
cargo test -p platform-esp32 --test climate_bridge
```

## Simulator / firmware の使い分け

```bash
# ClimateDisplayApp の terminal 16x2 表示
cargo run -p platform-pc-sim --bin climate-display-sim

# 配線 view 付き terminal dashboard
cargo run -p platform-pc-sim --bin climate-dashboard-sim
cargo run -p platform-pc-sim --bin climate-dashboard-sim -- nano

# browser dashboard
cargo run -p platform-pc-sim --bin device-dashboard-web
cargo run -p platform-pc-sim --bin device-dashboard-web -- nano 7878

# original ESP32 bring-up
cd firmware/original-esp32-bringup
cargo run --release

# original ESP32 climate display
cd firmware/original-esp32-climate-display
cargo check --release
cargo run --release
```

## original ESP32 reference path

`core_app::climate_display::ClimateDisplayApp` は次の 2 経路で共通利用します。

- PC simulator
  - `platform-pc-sim::climate_sim::{SequenceEnvSensor, TerminalDisplay16x2}`
  - `platform-pc-sim::{virtual_i2c::VirtualI2cBus, bme280_mock::MockBme280Device, lcd1602_mock::MockLcd1602Device}`
- original ESP32
  - `platform-esp32::{Bme280Sensor, Lcd1602Display, SharedI2cBus}`
  - firmware: `firmware/original-esp32-climate-display`

既定の実機想定:

- board: original ESP32 / ESP32-WROOM-32 系
- I2C: `GPIO21` = SDA, `GPIO22` = SCL
- BME280: `0x77` 優先、必要に応じて `0x76`
- LCD1602 backpack: `0x27`

## 実装ルール

- `core-app` に board 固有分岐を入れない
- board / sensor / display 差分は config struct に閉じ込める
- `reference-drivers` は board 非依存を維持する
- `platform-esp32` は adapter / re-export / type alias を中心に薄く保つ
- host simulator で先に contract を固定してから実機経路に接続する
- 実機手順や wiring 前提を変えたら README / PLAN / firmware README を同期する

## テスト方針

- 新規 trait / driver / adapter には正常系だけでなく error propagation のテストを追加する
- `ClimateDisplayApp` は sensor error / display error / tick scheduling / frame formatting を回帰テストで固定する
- BME280 / LCD1602 / SharedI2cBus は I2C error mapping と fallback behavior をテストする
- `no_std` 対象 crate に `std` 依存を漏らさない。テスト helper は `#[cfg(test)] extern crate std;` に閉じ込める

## PR / CI 運用

PR 前に最低限次を実行します。

```bash
cargo fmt --all -- --check
cargo clippy --all --all-targets -- -D warnings
cargo test --workspace --all-targets
```

reference path / ESP32 周辺を触った場合は、可能なら以下も確認します。

```bash
cargo check-esp32
./scripts/ci-local.sh
```

AI Agent レビューと CI が問題なければ、低リスクなテスト・ドキュメント・小規模修正 PR はマージ可能です。重要PR（実機破壊リスク、secrets/auth/security、破壊的設計変更、大規模アーキテクチャ変更など）はユーザーへエスカレーションします。

## 重要な原則

このプロジェクトでは **TDD / 小さなPR / sim-to-real contract の維持** を優先します。

- Red: 期待する contract をテストで固定
- Green: 最小限の実装で通す
- Refactor: `core-app` と platform 層の責務分離を維持

詳細な全体方針は `PLAN.md` と `README.md` も参照してください。
