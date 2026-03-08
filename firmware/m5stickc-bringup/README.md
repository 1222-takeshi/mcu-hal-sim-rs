# m5stickc-bringup

M5StickC 向けの board bring-up firmware です。

この crate は `core-app` を直接動かす用途ではなく、M5StickC 固有のハードウェアが
見えているかを最短で確認するための実機診断用として使います。

対象:

- Button A (`GPIO37`)
- Button B (`GPIO39`)
- onboard I2C bus (`GPIO21` / `GPIO22`)
- AXP192 (`0x34`)
- BM8563 (`0x51`)
- MPU6886 (`0x68`) または SH200Q (`0x6c`)

## 前提

- ボード想定: original M5StickC
- toolchain: `espup` で導入した `esp` toolchain
- フラッシュ: `espflash`
- 電圧: USB 給電または内蔵バッテリ
- 想定ホスト OS: native macOS / native Linux / Windows / WSL2

## 実行

```bash
cd firmware/m5stickc-bringup
cargo run --release

# host 側で判定ロジックだけを確認する
cargo test
```

WSL2 で serial port が Linux 側に見えない場合は、`cargo build --release` を WSL 側で行い、
flash / monitor は Windows 側の `espflash.exe` から実行してください。考え方は
`firmware/original-esp32-bringup` と同じです。

## ログで確認すること

- 起動時に M5StickC の pinmap を表示
- AXP192 / BM8563 / MPU6886 / SH200Q の probe 結果
- `probe summary: ...` で、PMU / RTC / IMU の応答有無を1行で確認
- `board status: ...` で、PMU/RTC、IMU、外付け BME280、onboard I2C の要約を確認
- `board hint: ...` で、次に疑うべき異常要因を確認
- Button A / Button B の押下・解放イベント
- Button A / Button B の初期状態
- heartbeat
- `bus health: ...` による定期的な AXP192 再確認

## 期待される観察ポイント

- `AXP192` と `BM8563` は通常 `yes` になる
- IMU はロット差があるため、`MPU6886` または `SH200Q` のどちらか一方が `yes` なら正常寄り
- `probe summary: expected exactly one IMU variant ...` が出る場合は、IMU 未応答か、配線/電源/I2C の異常を疑う
- ボタン未操作時は `button: A initial released`, `button: B initial released` が基本
- ボタンを押すと `button: A pressed (loop=...)` のような遷移ログが出る
- 起動後も `bus health: loop=... AXP192 ack ...` が継続して出れば、onboard I2C が落ちていない目安になる
- heartbeat は busy-wait ベースの概算です。厳密な周期ではなく、button polling と serial log が継続していることの確認に使ってください

## ホスト別メモ

- macOS: `/dev/cu.*` または `/dev/tty.*` を前提に `espflash` を使う
- Linux: `/dev/ttyUSB*` または `/dev/ttyACM*`
- Windows: `COMx`
- WSL2: build は WSL、flash / monitor は Windows 側に逃がすのが安定

## 注意

- M5StickC の IMU はロットにより `SH200Q` または `MPU6886` です
- board 上の red LED は `GPIO10` ですが、現行の `esp-hal` safe API では `esp32` target に露出していません
- そのため、この firmware はまず Button / I2C bring-up を優先しています
- Windows では公式ドキュメント上、USB driver は FTDI 系として案内されています
- `cargo run` がホスト側から serial port を見つけられない場合は、original ESP32 bring-up と同様に
  WSL で build して Windows 側の `espflash.exe` で flash / monitor してください
