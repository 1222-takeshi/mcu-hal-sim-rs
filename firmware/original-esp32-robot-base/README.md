# original-esp32-robot-base

ESP32 ロボットベース ファームウェアスケルトン。

`platform-esp32` の [`Esp32ServoDriver`] / [`Esp32L298nDualDriverSimple`] 型エイリアスを使った
サーボモータ + デュアルモータドライバの実機接続例です。

## 機能

- サーボモータ（SG90 等）を LEDC PWM で 0°〜180° 制御
- L298N デュアルモータドライバを GPIO + LEDC PWM で制御
- デモシーケンス（前進 → 旋回 → 停止 → 後退）をループ再生

## アーキテクチャ

```
esp_hal::ledc::Channel ──→ Esp32PwmOutput ──→ Esp32ServoDriver
esp_hal::gpio::Output  ──→ Esp32OutputPin  ──┐
                                              ├─→ Esp32L298nChannel ──→ Esp32L298nDualDriverSimple
esp_hal::ledc::Channel ──→ Esp32PwmOutput  ──┘
```

`Esp32ServoDriver` と `Esp32L298nDualDriverSimple` は
`crates/platform-esp32/types.rs` で定義された型エイリアスです。

## 配線

### サーボモータ（SG90 等）

| 信号 | ESP32 GPIO | 備考 |
|------|-----------|------|
| PWM  | GPIO 18   | LEDC Ch0, 50 Hz, 14-bit |

### L298N デュアルモータドライバ

| 信号  | ESP32 GPIO | 備考 |
|------|-----------|------|
| IN1-A | GPIO 25 | チャンネル A 方向 1 |
| IN2-A | GPIO 26 | チャンネル A 方向 2 |
| ENA   | GPIO 27 | チャンネル A PWM (LEDC Ch1, 1 kHz) |
| IN1-B | GPIO 32 | チャンネル B 方向 1 |
| IN2-B | GPIO 33 | チャンネル B 方向 2 |
| ENB   | GPIO 14 | チャンネル B PWM (LEDC Ch2, 1 kHz) |

> **注**: L298N の 5 V 論理ラインは ESP32 の 3.3 V GPIO と直結できます（L298N の入力は 3 V〜5 V 対応）。

### 電源

| ライン | 電圧 |
|--------|------|
| ESP32  | 5 V（USB または VIN） |
| サーボ | 5 V（ESP32 とは別電源推奨） |
| L298N  | 6〜12 V（モータ電源） |
| L298N 5 V | ESP32 の 5 V ピンへ供給可 |

## 必要なツール

```bash
# xtensa ツールチェーンのインストール（初回のみ）
cargo install espup
espup install
source $HOME/export-esp.sh

# espflash のインストール
cargo install espflash
```

## ビルドと書き込み

```bash
# ワークスペースルートから scripts/flash-esp32.sh を使う場合（推奨）
cd /path/to/mcu-hal-sim-rs
./scripts/flash-esp32.sh original-esp32-robot-base           # ポート自動検出
./scripts/flash-esp32.sh original-esp32-robot-base /dev/cu.usbserial-0001  # ポート明示

# または firmware ディレクトリ内で直接実行する場合
cd firmware/original-esp32-robot-base

# ビルドのみ
cargo build --release

# フラッシュ書き込み + シリアルモニタ（macOS: /dev/cu.*）
cargo run --release
```

`flash-esp32.sh` はポートを自動検出します:
- macOS: `/dev/cu.usbserial-*`, `/dev/cu.SLAB_*`, `/dev/cu.wchusbserial-*` 等
- Linux: `/dev/ttyUSB*`, `/dev/ttyACM*`
- WSL2: `espflash.exe` 経由で Windows COM ポートを使用（`ESP32_PORT=COM3 ./scripts/flash-esp32.sh ...`）

## 期待される出力

```
=== ESP32 Robot Base Firmware ===
Servo: GPIO 18
Motor A: IN1=25 IN2=26 ENA=27
Motor B: IN1=32 IN2=33 ENB=14
[tick 0] step 0 — servo=0° Forward@50%
[tick 50] step 1 — servo=45° Forward@75%
[tick 100] step 2 — servo=90° Brake@50%
[tick 150] step 3 — servo=135° Reverse@60%
[tick 200] step 4 — servo=180° Reverse@40%
[tick 250] step 5 — servo=90° Brake@0%
```

## 関連

- [`crates/platform-esp32/types.rs`](../../crates/platform-esp32/types.rs) — 型エイリアス定義
- [`firmware/original-esp32-climate-display`](../original-esp32-climate-display/) — センサー版（I2C）
- [`crates/platform-esp32/tests/servo_motor_bridge.rs`](../../crates/platform-esp32/tests/servo_motor_bridge.rs) — ブリッジテスト
