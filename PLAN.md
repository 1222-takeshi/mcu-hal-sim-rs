# mcu-hal-sim-rs 開発プラン

## 概要

`mcu-hal-sim-rs` は、マイコン向け Rust アプリを **PC simulator で検証し、そのまま実機へ持っていくための基盤 repo** です。
現時点の reference path は original ESP32 ですが、将来的な `Arduino Nano` / `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` 展開を見据え、`hal-api` / `core-app` / `platform-*` の責務分離と sim-to-real 契約の固定を主目的にします。

## 現状（2026-03 時点）

### 成立している本線

- `platform-pc-sim` 上で `ClimateDisplayApp` を動かせる
- `platform-esp32` を通じて original ESP32 上で `ClimateDisplayApp` を動かせる
- `BME280 + LCD1602` の climate display を PC simulator と original ESP32 実機で共通ロジックのまま確認できる
- `hal-api` と `core-app` は `no_std` を維持できている
- CI / テスト / ドキュメントが基盤として成立している

### 補助的に持っているもの

- `firmware/original-esp32-bringup`
  - USB / flash / LED / 汎用 I2C の切り分け
- `firmware/m5stickc-bringup`
  - M5StickC を使った USB / button / onboard I2C の診断
- `firmware/arduino-nano-bringup`
  - classic Arduino Nano (`ATmega328P`) の LED / serial / I2C scan の切り分け

## この repo のスコープ

### 本 repo に残すもの

- `hal-api` の汎用抽象
- `core-app` の再利用可能なアプリロジック
- `platform-pc-sim` / `platform-avr` / `platform-esp32` の sim-to-real 経路
- original ESP32 を使った本線シナリオの維持
- 再利用可能な sensor / display driver とそのテスト

### 本 repo に残さないもの

- 特定 board だけで完結する実験的 UI
- camera / wireless / board 固有周辺機能の寄せ集め
- 個別プロダクト向けのアプリ要件そのもの

これらは原則として別 repo で実装し、共通抽象が必要になった時点で本 repo に還流する。

## 開発方針

### 1. reference path を安定面として維持する

- 主経路は `platform-pc-sim -> core-app -> platform-esp32 -> original ESP32 + BME280 + LCD1602`
- M5StickC は補助診断ボードであり、本番経路には含めない
- 新しい board を足すときも、まず既存 reference path を壊さないことを優先する

### 2. 実アプリと board 固有の検証は外で育てる

- 新しいマイコンアプリは別 repo で作る
- board 固有の bring-up や camera のような強い周辺機能も、まず firmware / 別 repo で検証する
- 本 repo は `git` 依存または path 依存で利用する
- 別 repo で必要になった抽象だけを本 repo に戻す

### 3. 追加判断は「共通化価値」で行う

次のいずれかを満たす場合だけ、本 repo へ取り込む。

- `hal-api` の抽象が他案件でも再利用できる
- `core-app` の再利用性を上げられる
- `platform-pc-sim` と `platform-esp32` の差分を減らせる
- sim-to-real の検証価値がある
- 将来の別 board / sensor 追加で、そのまま contract として使い回せる

## 直近の優先タスク

### A. スコープの固定

- `README` / `PLAN` / AI コンテキストを現状に揃え続ける
- 本 repo が基盤 repo であることを明文化する

### B. 本線の品質維持と拡張しやすさの両立

- `ClimateDisplayApp` の回帰テストを増やす
- BME280 / LCD1602 / shared I2C の異常系を補強する
- 実機確認済みの手順をドキュメントへ維持する
- app / sensor / display 差分を config struct で表現し、board 固有差分を core へ漏らしにくくする

### C. 拡張候補の評価

- 次のマイコン案件は別 repo で開始する
- その repo から本 repo を依存として利用する
- 共通化が必要な時だけ本 repo に PR を戻す
- `Arduino Nano` / `Raspberry Pi Pico` / `Teensy` / `ESP32-CAM` の候補は、reference path を壊さない単位で platform / firmware を試作する
- sensor lineup は、まず `EnvSensor` に載るものから増やす
- その中では、classic Arduino Nano を AVR 系の最初の bring-up 起点とする
- `platform-avr` は generic adapter 層として維持し、board helper は必要になった時だけ追加する

## esp32cam の扱い

- `esp32cam` は camera・frame buffer・board 固有配線を含むため、現時点では **先に本 repo へ追加しない**
- まずは別 repo で `esp32cam` 向けアプリや bring-up を作る
- その作業で、camera 抽象や board 非依存の画像取得 API が本当に必要だと分かった場合だけ、本 repo に最小限の抽象を追加する
- つまり、`esp32cam` は **先に別 repo**、本 repo への還流は **後から必要最小限** が基本方針

## 受け入れ基準

- `cargo test --workspace --all-targets` が通る
- 本線シナリオの docs が current state と矛盾しない
- 新規追加が本 repo のスコープ説明と整合する
- board 固有機能を追加する場合、別 repo ではなく本 repo に置く理由を説明できる
