# mcu-hal-sim-rs

[![CI](https://github.com/1222-takeshi/mcu-hal-sim-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/1222-takeshi/mcu-hal-sim-rs/actions/workflows/ci.yml)

マイコン向け Rust アプリケーションを、**ハードウェア抽象化層（HAL）**を通じてプラットフォーム非依存に記述し、PC 上のシミュレータで動作確認したうえで実機へ持っていくための基盤プロジェクトです。

## 🎯 スコープ

- 本リポジトリの主目的は、**sim-to-real 経路を成立させるための reference implementation を育てること** です。
- 現在の reference path は **`platform-pc-sim -> core-app -> platform-esp32 -> original ESP32 + BME280 + LCD1602`** です。
- `M5StickC` は本番ターゲットではなく、USB / button / onboard I2C を素早く切り分けるための **補助診断ボード** として扱います。
- `Arduino Nano` / `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` のような将来ターゲットを意識しつつ、追加は **契約の共通化価値があるもの** に絞ります。
- 新しい board / sensor / display は、まず firmware / platform 層か別 repo で検証し、`hal-api` または `core-app` に戻す価値がある契約だけを還流します。

## ✨ 特徴

- **🎯 プラットフォーム非依存**: HAL traitを使用し、同じアプリケーションコードを複数のプラットフォームで実行
- **💻 PCシミュレータ**: 実機なしで開発・デバッグが可能
- **🧪 テスト駆動開発**: workspace 全体で unit / integration / doc test を継続実行
- **🔧 CI/CD自動化**: GitHub Actionsで自動ビルド・テスト・Lint
- **📦 `no_std` 準備**: `hal-api` と `core-app` はホスト依存を持たない構成
- **⚙️ original ESP32 実装**: `platform-esp32` は `embedded-hal` v1.0 経由で実機 HAL を受けられる構成
- **🧪 board bring-up を独立化**: ESP32 に加えて classic Arduino Nano 向け bring-up firmware を追加
- **🧩 拡張しやすい設定モデル**: app / sensor / display の設定を config struct で公開
- **🧰 実機雛形あり**: `firmware/original-esp32-bringup` から LED only / real I2C の両方を試せる
- **🖥️ LCD simulation UI**: `climate-display-sim` で 16x2 表示を terminal 上にそのまま確認できる
- **🧪 device-level host mock**: `platform-pc-sim` で virtual I2C bus と `BME280` mock device を使い、実 driver を host 上で検証できる
- **📊 terminal dashboard**: `climate-dashboard-sim` で sensor / LCD / I2C / wiring view を 1 画面で確認できる
- **🌐 browser dashboard**: `device-dashboard-web` で climate / HC-SR04 / MPU6050 / servo / motor driver をブラウザで可視化できる
- **🧪 IMU driver bridge**: `MPU6050` は host-side mock device と reference driver を経由して GUI / test に載せられる
- **📏 distance driver bridge**: `HC-SR04` は pulse/echo mock device と reference driver を経由して GUI / test に載せられる
- **🧱 sensor / actuator 契約を拡張**: `hal-api` に distance / IMU / servo / drive motor / dual motor driver の board 非依存 trait を追加
- **🌡️ Sim-to-real 経路**: `ClimateDisplayApp` を PC simulator と original ESP32 実機で再利用
- **📡 観測しやすい実機ログ**: climate display firmware が frame と sensor 値を serial log に出力
- **🎛️ 補助診断 board**: `firmware/m5stickc-bringup` で M5StickC の Button / PMU / RTC / IMU を切り分け可能
- **📦 downstream 利用前提**: 実アプリは別リポジトリから `git` / `path` 依存で利用し、共通化すべき抽象だけを本 repo に戻す運用を前提化

## 📐 アーキテクチャ

```
┌─────────────────────────────────────────────┐
│          Application Layer                  │
│  ┌────────────────────────────────────┐     │
│  │  core-app                          │     │
│  │  (プラットフォーム非依存ロジック)  │     │
│  └─────────────┬──────────────────────┘     │
└────────────────┼──────────────────────────────┘
                 │ depends on
┌────────────────▼──────────────────────────────┐
│          HAL Trait Layer                      │
│  ┌────────────────────────────────────┐       │
│  │  hal-api                           │       │
│  │  - OutputPin trait (GPIO)          │       │
│  │  - I2cBus trait (I2C)              │       │
│  └────────────────────────────────────┘       │
└────────────────┬──────────────────────────────┘
                 │ implemented by
        ┌────────┴────────┐
        │                 │
┌───────▼──────┐  ┌──────▼────────────────┐
│ PC Simulator │  │ ESP32 (original)      │
│ platform-    │  │ platform-     │
│ pc-sim       │  │ esp32         │
│ - MockPin    │  │ - Esp32Pin    │
│ - MockI2c    │  │ - Esp32I2c    │
└──────────────┘  └───────────────────────┘
        │
 ┌──────▼───────────────┐
 │ AVR (planned path)   │
 │ platform-avr         │
 │ - AvrOutputPin       │
 │ - AvrI2c             │
 └──────────────────────┘
```

この基本構造に加えて、`hal-api` は `EnvSensor` / `TextDisplay16x2` に加えて
`DistanceSensor` / `ImuSensor` / `ServoMotor` / `DriveMotor` / `DualMotorDriver` を公開し、
board 固有の HAL から sensor / actuator を切り離せるようにしています。

現在の reference app は `ClimateDisplayApp` ですが、browser dashboard 側では
`HC-SR04` / `MPU6050` / servo / dual motor driver を同じ host rig 上で組み合わせて可視化できます。

## 🧭 運用方針

- 本リポジトリは **基盤 repo** として扱います。
- 実際のマイコン向けアプリケーションは別 repo で作り、必要に応じてこの repo を `git` 依存または path 依存で利用します。
- 別 repo から本 repo へ戻す変更は、次のいずれかを満たすものに限定します。
  - `hal-api` の抽象を汎用化できる
  - `core-app` の再利用性を上げられる
  - `platform-pc-sim` と `platform-esp32` の sim-to-real 経路を改善できる
- board 固有の UI、camera、通信機能のように他案件へ波及しにくい実装は、まず firmware / 別 repo で検証する方針です。
- 詳細な追加ルールは [docs/porting-and-extension-guide.md](./docs/porting-and-extension-guide.md) を参照してください。

## 🚀 クイックスタート

### 前提条件

- Rust 1.66以降（[rustup](https://rustup.rs/)でインストール）

### ビルド

```bash
# プロジェクトをクローン
git clone https://github.com/1222-takeshi/mcu-hal-sim-rs.git
cd mcu-hal-sim-rs

# すべてのクレートをビルド
cargo build

# リリースビルド（最適化あり）
cargo build --release
```

### downstream 利用

別 repo から最短で試す場合は、まず `hal-api` / `core-app` / `platform-pc-sim` を path 依存か `git` 依存で読み込みます。

```toml
[dependencies]
hal-api = { git = "https://github.com/1222-takeshi/mcu-hal-sim-rs.git" }
core-app = { git = "https://github.com/1222-takeshi/mcu-hal-sim-rs.git" }
platform-pc-sim = { git = "https://github.com/1222-takeshi/mcu-hal-sim-rs.git" }
```

### 実行

```bash
# PCシミュレータを実行
cargo run -p platform-pc-sim

# BME280 + LCD1602 の climate display シミュレータを実行
cargo run -p platform-pc-sim --bin climate-display-sim

# 配線 view 付き dashboard を実行
cargo run -p platform-pc-sim --bin climate-dashboard-sim

# Nano profile で dashboard を実行
cargo run -p platform-pc-sim --bin climate-dashboard-sim -- nano

# browser GUI を起動（デフォルト: ESP32 profile、port 7878）
cargo run -p platform-pc-sim --bin device-dashboard-web

# Nano profile + port 指定で browser GUI を起動
cargo run -p platform-pc-sim --bin device-dashboard-web -- nano 7878
```

`climate-display-sim` では terminal 上に次のような 16x2 表示を描画します。

```text
+----------------+
|Temp    24.8C   |
|Hum     43.1%   |
+----------------+
```

### ブラウザ GUI 動作確認 (device-dashboard-web)

```bash
cargo run -p platform-pc-sim --bin device-dashboard-web
# → http://127.0.0.1:7878 をブラウザで開く
```

起動後、以下のパネルで動作を確認できます。

| パネル | 確認ポイント |
|--------|------------|
| **ヘッダー** | `Board: original ESP32 / tick=N` が 250 ms ごとに増える |
| **Climate** | 温度・湿度・気圧の数値とスパークライン |
| **LCD1602** | 2 行 16 文字の liquid crystal 風テキスト表示 |
| **Distance** | HC-SR04 ソナーの扇形ビーム（近い=赤、遠い=緑） |
| **IMU** | 加速度に連動する水準器バブル |
| **LED** | tick に合わせて 100 tick ごとに輝く |
| **Servo** | 距離に応じてアームが 0〜180° で動く |
| **Motor L/R** | 距離 < 160 mm → Reverse、それ以外 → Forward で回転 |
| **Wiring Diagram** | PCB 風 SVG。I2C 操作のたびに SDA/SCL ラインが白く光る |
| **Board セレクター** | "Arduino Nano" に切り替えると配線 SVG のピン名が変わる |
| **E2E Test Runner** | "▶ Run Tests" を押すと `cargo test --workspace` がリアルタイムにストリーミングされる |

**API による確認:**

```bash
# 現在の状態 JSON
curl http://127.0.0.1:7878/api/state | python3 -m json.tool

# 配線設定 JSON (GET)
curl http://127.0.0.1:7878/api/wiring | python3 -m json.tool

# ボードプロファイル切り替え (POST)
curl -X POST http://127.0.0.1:7878/api/wiring \
     -H "Content-Type: application/json" \
     -d '{"board":"arduino-nano"}' | python3 -m json.tool

# 配線 SVG を取得してファイルに保存
curl http://127.0.0.1:7878/api/wiring/svg -o wiring.svg
```

有効な board 値: `"original-esp32"` (デフォルト)、`"arduino-nano"`

### テスト

```bash
# すべてのテストを実行
cargo test --workspace

# 詳細出力
cargo test --workspace -- --nocapture

# 特定のクレートのみ
cargo test -p core-app
```

### コード品質チェック

```bash
# すべてのCIチェックをローカルで実行（推奨）
./scripts/ci-local.sh

# 自動修正モード
./scripts/ci-local.sh --fix

# 個別チェック
cargo fmt --all -- --check            # フォーマットチェック
cargo clippy --all --all-targets -- -D warnings  # Lintチェック
cargo check -p hal-api --lib --target thumbv6m-none-eabi
cargo check -p core-app --lib --target thumbv6m-none-eabi
cargo check -p platform-esp32 --lib --target thumbv6m-none-eabi
cargo check-esp32
```

### original ESP32 実機向けの最小確認

`platform-esp32` は original Xtensa-based ESP32 を対象に進めています。

```bash
# 1. Xtensa 向け toolchain をセットアップ
#    https://docs.espressif.com/projects/rust/book/

# 2. 実機向けチェック
cargo check-esp32
```

詳細は [crates/platform-esp32/README.md](./crates/platform-esp32/README.md) を参照してください。

### original ESP32 bring-up

![Original ESP32 bring-up flow](./docs/images/original-esp32-bringup-flow.svg)

![Original ESP32 wiring](./docs/images/original-esp32-wiring.svg)

実機 bring-up は [firmware/original-esp32-bringup/README.md](./firmware/original-esp32-bringup/README.md) から始めてください。

```bash
# LED だけ先に確認
cd firmware/original-esp32-bringup
cargo run --release

# 0x48 の I2C デバイスがある場合
cargo run --release --features real-i2c
```

現在の `core-app` は `0x48` に 4-byte read を行うため、I2C を試す場合は `0x48` で応答する 3.3V デバイスが必要です。
実行ホスト OS は native macOS / native Linux / Windows / WSL2 を想定します。
original ESP32 + CP210x bridge では、LED only firmware の flash / boot log まで確認済みです。
WSL2 で `/dev/ttyUSB*` が見えない場合は、WSL で build して Windows 側の `espflash.exe` から `COMx` へ書き込む手順を使ってください。
macOS では Windows の `COMx` ではなくネイティブの serial device path を前提にしてください。

### Climate display の sim-to-real 経路

![Climate display sim-to-real](./docs/images/climate-display-sim-to-real.svg)

`core_app::climate_display::ClimateDisplayApp` は、次の 2 経路で共通利用します。

- PC:
  - `platform-pc-sim::climate_sim::{SequenceEnvSensor, TerminalDisplay16x2}`
  - `platform-pc-sim::{virtual_i2c::VirtualI2cBus, bme280_mock::MockBme280Device}`
- original ESP32: `platform-esp32::{Bme280Sensor, Lcd1602Display, SharedI2cBus}`

実機 firmware は [firmware/original-esp32-climate-display/README.md](./firmware/original-esp32-climate-display/README.md) を参照してください。

```bash
cd firmware/original-esp32-climate-display

# 型検査
cargo check --release

# toolchain / linker が揃っていれば flash まで
cargo run --release
```

### M5StickC を診断用 board として使う

M5StickC は `core-app` の本番実行先というより、ESP32 系 board の I2C / button / USB 接続を
最短で切り分けるための診断用として位置づけています。

```bash
cd firmware/m5stickc-bringup
cargo run --release
```

詳細は [firmware/m5stickc-bringup/README.md](./firmware/m5stickc-bringup/README.md) を参照してください。

この時点での使い分けは次の通りです。

- `cargo run -p platform-pc-sim --bin climate-display-sim`
  - 表示文言、更新周期、16x2 UI を host 上で最初に詰めるとき
- `cargo run -p platform-pc-sim --bin climate-dashboard-sim`
  - wiring / LCD / I2C を terminal 上で速く確認したいとき
- `cargo run -p platform-pc-sim --bin device-dashboard-web`
  - 全センサ・アクチュエータのビジュアルシミュレータを browser で確認したいとき（v0.3.0〜）
- `cargo run -p platform-pc-sim --bin device-dashboard-web -- nano 7878`
  - Arduino Nano profile で同じ GUI を確認したいとき
- `firmware/original-esp32-bringup`
  - USB / flash / basic GPIO / 汎用 I2C 疎通だけを切り分けたいとき
- `firmware/original-esp32-climate-display`
  - `BME280 + LCD1602` の本命経路を original ESP32 で確認したいとき
- `firmware/m5stickc-bringup`
  - M5StickC を第2の診断ボードとして使い、Button / onboard I2C デバイスを確認したいとき

`M5StickC` は climate display の本命 board ではなく、USB / button / onboard I2C の切り分けを早く回すための補助ボードとして位置付けています。

### classic Arduino Nano bring-up

classic Arduino Nano (`ATmega328P`) 向けの最小 bring-up も追加しています。

```bash
cd firmware/arduino-nano-bringup
cargo run --release
```

詳細は [firmware/arduino-nano-bringup/README.md](./firmware/arduino-nano-bringup/README.md) を参照してください。

この firmware は `D13` の LED 点滅、USB serial、`A4/A5` の I2C scan を確認するためのもので、将来の `platform-avr` や sensor 追加の前段に位置づけています。

### CI結果の自動監視

PRをプッシュした後、CIの完了を自動で監視:

```bash
# 最新のワークフローを監視
./scripts/ci-wait.sh

# 特定のrun-idを監視
./scripts/ci-wait.sh 21797882688
```

## 📦 プロジェクト構成

```
mcu-hal-sim-rs/
├── crates/
│   ├── hal-api/          # HAL trait定義
│   │   ├── README.md     # crate overview
│   │   ├── actuator.rs   # ServoMotor / DriveMotor / DualMotorDriver
│   │   ├── distance.rs   # DistanceSensor
│   │   ├── display.rs    # 16x2 表示 trait
│   │   ├── error.rs      # エラー型（GPIO / I2C / sensor / display）
│   │   ├── gpio.rs       # GPIO trait（OutputPin, InputPin）
│   │   ├── i2c.rs        # I2C trait（I2cBus）
│   │   ├── imu.rs        # ImuSensor
│   │   ├── sensor.rs     # 環境センサ trait
│   │   └── lib.rs
│   │
│   ├── core-app/         # アプリケーションロジック
│   │   ├── README.md           # crate overview
│   │   ├── climate_display.rs  # ClimateDisplayApp
│   │   └── lib.rs              # App<PIN, I2C>構造体
│   │                           #   - 100 tickごとにLED点滅
│   │                           #   - 500 tickごとにI2C読み取り
│   │
│   ├── reference-drivers/ # board非依存の reference driver
│   │   ├── README.md
│   │   ├── bme280.rs          # BME280 sensor driver
│   │   ├── hc_sr04.rs         # HC-SR04 distance driver
│   │   ├── lcd1602.rs         # LCD1602 backpack driver
│   │   ├── mpu6050.rs         # MPU6050 IMU driver
│   │   └── lib.rs
│   │
│   ├── platform-pc-sim/  # PCシミュレータ
│   │   ├── README.md              # crate overview
│   │   ├── bme280_mock.rs         # host-side BME280 mock device
│   │   ├── climate_dashboard_sim.rs # terminal dashboard demo
│   │   ├── climate_sim.rs         # SequenceEnvSensor / TerminalDisplay16x2
│   │   ├── climate_display_sim.rs # 16x2 terminal demo
│   │   ├── component_sim.rs       # HC-SR04 / MPU6050 / actuator simulator
│   │   ├── dashboard.rs           # dashboard renderer / board profiles
│   │   ├── device_dashboard_web.rs # browser dashboard server
│   │   ├── hc_sr04_mock.rs        # host-side HC-SR04 pulse/echo mock
│   │   ├── lib.rs                 # モックHAL公開
│   │   ├── lcd1602_mock.rs        # host-side LCD1602 mock device
│   │   ├── main.rs                # エントリポイント（10ms tickループ）
│   │   ├── mock_hal.rs            # モックHAL実装
│   │   ├── mpu6050_mock.rs        # host-side MPU6050 mock device
│   │   ├── virtual_i2c.rs         # host-side virtual I2C bus
│   │   └── web_dashboard.rs       # browser UI HTML / JSON state
│
│   ├── platform-avr/      # AVR系向けアダプタ
│   │   ├── README.md
│   │   ├── gpio.rs          # AvrOutputPin / AvrInputPin
│   │   ├── i2c.rs           # AvrI2c
│   │   ├── lib.rs
│   │   └── tests/app_bridge.rs
│
│   └── platform-esp32/   # original ESP32向けアダプタ
│       ├── bme280.rs       # reference-drivers の re-export
│       ├── gpio.rs         # Esp32OutputPin / Esp32InputPin
│       ├── hc_sr04.rs      # reference-drivers の re-export
│       ├── i2c.rs          # Esp32I2c
│       ├── lcd1602.rs      # reference-drivers の re-export
│       ├── mpu6050.rs      # reference-drivers の re-export
│       ├── shared_i2c.rs   # 共有 I2C バス
│       ├── lib.rs
│       └── README.md
│
├── docs/
│   ├── images/                # 配線図 / bring-up フロー図
│   └── porting-and-extension-guide.md
│
├── .github/
│   └── workflows/
│       └── ci.yml        # CI/CD設定
│
├── .cargo/
│   └── config.toml       # original ESP32向け cargo alias / runner
│
├── firmware/
│   ├── original-esp32-bringup/
│   │   ├── .cargo/config.toml  # xtensa target / espflash runner
│   │   ├── src/main.rs         # LED only / real I2C bring-up
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── rust-toolchain.toml
│   ├── m5stickc-bringup/
│   │   ├── .cargo/config.toml  # xtensa target / espflash runner
│   │   ├── src/main.rs         # Button / onboard I2C diagnostics
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── rust-toolchain.toml
│   ├── arduino-nano-bringup/
│   │   ├── .cargo/config.toml  # avr target / ravedude runner
│   │   ├── src/main.rs         # D13 blink + serial + I2C detect
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── rust-toolchain.toml
│   └── original-esp32-climate-display/
│       ├── .cargo/config.toml  # xtensa target / espflash runner
│       ├── src/main.rs         # BME280 + LCD1602 climate display
│       ├── Cargo.toml
│       ├── README.md
│       └── rust-toolchain.toml
│
├── Cargo.toml            # ワークスペース設定
├── rustfmt.toml          # フォーマット設定
└── README.md             # このファイル
```

### クレートの役割

| クレート | 説明 | 依存関係 |
|---------|------|---------|
| **hal-api** | HAL trait定義（`OutputPin`, `I2cBus`, `EnvSensor`, `DistanceSensor`, `ImuSensor`, `ServoMotor`, `DualMotorDriver`, `TextDisplay16x2` など） | なし |
| **core-app** | プラットフォーム非依存のアプリケーションロジック（`App`, `ClimateDisplayApp`） | `hal-api` |
| **reference-drivers** | board 非依存の reference sensor / distance / display / IMU driver | `hal-api`, `embedded-hal` |
| **platform-pc-sim** | PCシミュレータ実装（モックHAL + virtual I2C + host-side mock device + terminal / browser dashboard） | `hal-api`, `core-app`, `reference-drivers` |
| **platform-avr** | AVR系向け `embedded-hal` アダプタ | `hal-api`, `embedded-hal` |
| **platform-esp32** | original ESP32向け `embedded-hal` アダプタ + reference driver re-export | `hal-api`, `embedded-hal`, `reference-drivers` |

### 実機用テンプレート

| ディレクトリ | 説明 | 依存関係 |
|-------------|------|---------|
| **firmware/original-esp32-bringup** | original ESP32 向け bring-up 雛形 | `core-app`, `platform-esp32`, `esp-hal` |
| **firmware/m5stickc-bringup** | M5StickC 向け board diagnostics | `platform-esp32`, `esp-hal` |
| **firmware/arduino-nano-bringup** | classic Arduino Nano 向け blink / serial / I2C bring-up | `arduino-hal` |
| **firmware/original-esp32-climate-display** | BME280 + LCD1602 向け climate display firmware | `core-app`, `platform-esp32`, `esp-hal` |

## 🧪 テスト

このプロジェクトはテスト駆動開発（TDD）で構築されています。

| クレート | テストタイプ | テスト数 |
|---------|------------|---------|
| hal-api | ユニット + doc test | 2個 |
| core-app | ユニット + doc test | 31個 |
| reference-drivers | ユニットテスト | 18個 |
| platform-pc-sim | ユニット + 統合 + doc test | 34個 |
| platform-avr | ユニット + 統合テスト | 8個 |
| platform-esp32 | ユニット + 統合テスト | 14個 |
| **合計** | | **107個** |

## 🛠️ 開発

### TDD原則

このプロジェクトは以下のTDDサイクルに従います:

1. **🔴 Red**: 失敗するテストを書く
2. **🟢 Green**: テストを通すための最小限の実装
3. **🔵 Refactor**: コードを改善

詳細は [CLAUDE.md](./CLAUDE.md) を参照してください。

### コントリビューション

プルリクエストを歓迎します！詳細は [CONTRIBUTING.md](./CONTRIBUTING.md) をご覧ください。

**クイックスタート:**

1. このリポジトリをフォーク
2. 機能ブランチを作成 (`git checkout -b feat/amazing-feature`)
3. **🔴 Red**: テストを先に書く
4. **🟢 Green**: 実装してテストを通す
5. **🔵 Refactor**: コードを改善
6. 変更をコミット (`git commit -m 'feat: add amazing feature'`)
7. `./scripts/gh-workflow.sh push` でブランチをプッシュ
8. `./scripts/gh-workflow.sh pr -B main --fill` でプルリクエストを作成

開発ガイドライン、TDD原則、コーディング規約などの詳細は [CONTRIBUTING.md](./CONTRIBUTING.md) を参照してください。

## 📅 次にやること

- [ ] `ClimateDisplayApp` の reference path を保ちながら、新しい board / sensor の追加手順を標準化する
- [ ] `Arduino Nano` の bring-up から `platform-avr` へ還流できる共通 contract を切り出す
- [ ] `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` のような候補を、共通契約へ還元できる単位で検証する
- [ ] servo / motor driver の host-side mock を実 driver bridge まで広げる
- [ ] browser dashboard を WebSocket / canvas / wiring editor まで育てる
- [ ] `EnvSensor` 以外の sensor / actuator lineup も増やし、downstream repo が driver を差し替えやすい状態を作る
- [ ] publish 対象 crate の release 導線を固める

詳細は [CHANGELOG.md](./CHANGELOG.md) と [PLAN.md](./PLAN.md) を参照してください。

## 📄 ライセンス

このプロジェクトはMITライセンスの下で公開されています。

## 🙏 謝辞

このプロジェクトは、組み込みRustコミュニティの素晴らしい取り組み（特に[embedded-hal](https://github.com/rust-embedded/embedded-hal)）にインスパイアされています。
