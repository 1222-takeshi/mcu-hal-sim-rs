# raspi-pico-bringup

Raspberry Pi Pico 向けの bring-up firmware です。

`platform-rp2040` adapter を経由して `core-app` を動かし、
sim-to-real 経路の最初の接続確認を行います。

## 確認内容

1. LED 点滅（GPIO25 オンボード LED）
2. UART 出力（115200 baud, GPIO0=TX / GPIO1=RX）
3. I2C スキャン（I2C0, GPIO4=SDA / GPIO5=SCL）
4. `core-app::App` の tick ループ実行

## ピン配置

| 機能 | GPIO | 備考 |
|---|---|---|
| LED | GPIO25 | Pico オンボード LED |
| UART TX | GPIO0 | シリアルモニタに接続 |
| UART RX | GPIO1 | |
| I2C SDA | GPIO4 | I2C0 |
| I2C SCL | GPIO5 | I2C0 |

## 書き込み方法

### UF2 経由（推奨）

```bash
cd firmware/raspi-pico-bringup

# Pico を BOOTSEL ボタンを押しながら接続 → USB マスストレージとして認識
cargo run --release
# elf2uf2-rs が自動的に UF2 に変換して書き込みます
```

### 前提ツール

```bash
# elf2uf2-rs インストール
cargo install elf2uf2-rs

# thumbv6m-none-eabi ターゲット追加（rust-toolchain.toml で自動管理）
rustup target add thumbv6m-none-eabi
```

## シリアル出力例

```
raspi-pico bring-up + hal-api demo
LED=GPIO25 SDA=GPIO4 SCL=GPIO5 UART0=GPIO0/1
Use this firmware to confirm blink/UART/I2C before adding sensors.
I2C scan:
  found device at 0x27
  found device at 0x77
I2C scan done.
Starting core-app via hal-api adapters...
heartbeat
heartbeat
...
```
