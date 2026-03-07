# original-esp32-bringup

original ESP32 向けの最小ファームウェア雛形です。

この crate は workspace 外に置いてあり、ホスト CI には乗せずに実機 bring-up 専用として扱います。
ロジック本体は `core-app` をそのまま使い、GPIO / I2C だけを `platform-esp32` 経由で接続します。

![Original ESP32 wiring](../../docs/images/original-esp32-wiring.svg)

## 前提

- ボード想定: common ESP32 DevKitC / WROOM-32 系
- toolchain: `espup` で導入した `esp` toolchain
- フラッシュ: `espflash`
- 電圧: **3.3V only**

## まずやること

```bash
cd firmware/original-esp32-bringup

# LED だけで bring-up
cargo run --release
```

このモードでは GPIO2 の LED 点滅だけを確認し、I2C は no-op です。

## I2C も試す場合

`core-app` は 7-bit address `0x48` に対して 4-byte read を行います。
そのため、I2C bring-up では **0x48 で応答する 3.3V デバイス** を使ってください。

```bash
cd firmware/original-esp32-bringup
cargo run --release --features real-i2c
```

## 配線

### LED only

- `GPIO2` -> `220Ω` から `330Ω` の抵抗 -> LED アノード
- LED カソード -> `GND`

ボードに onboard LED がある場合は、その LED が `GPIO2` に載っていることがあります。
もし違う GPIO に載っている場合は `src/main.rs` の `LED_GPIO` と `peripherals.GPIO2` を合わせて変更してください。

### I2C

- `GPIO21` -> `SDA`
- `GPIO22` -> `SCL`
- `3V3` -> `VCC`
- `GND` -> `GND`
- `SDA` / `SCL` の pull-up がモジュールに無い場合は `4.7kΩ` を `3V3` へ追加

## 補足

- この crate の `.cargo/config.toml` は `xtensa-esp32-none-elf` を default target にしています
- `cargo run` で `espflash flash --monitor` が実行されます
- 初回で詰まりやすいのは toolchain と USB 接続です。USB ケーブルは給電専用でないものを使ってください
