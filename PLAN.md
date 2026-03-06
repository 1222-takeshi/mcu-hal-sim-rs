# mcu-hal-sim-rs 開発プラン

## 概要

`mcu-hal-sim-rs` は、GPIO / I2C を中心とした MCU 非依存の HAL trait と、
それを使うプラットフォーム非依存アプリケーションを Rust で構築し、
PC シミュレータで検証したうえで将来的に ESP32 実機へ展開するプロジェクトです。

現在は、PC シミュレータ基盤、テスト、CI、ドキュメントが揃っており、
次フェーズは「機能の初期実装」ではなく「品質を固めながら ESP32 適合性を高める」段階です。

---

## 現状（2026-03 時点）

### 完了済み

- Cargo workspace 構成
  - `crates/hal-api`
  - `crates/core-app`
  - `crates/platform-pc-sim`
- HAL API の定義
  - `OutputPin` / `InputPin`
  - `I2cBus`
  - `GpioError` / `I2cError`
- `core-app` のアプリケーションロジック
  - 100 tick ごとの LED 切り替え
  - 500 tick ごとの I2C 読み取り
  - `AppError` によるエラー伝播
- PC シミュレータ
  - `MockPin` / `MockI2c`
  - 10ms 周期のメインループ
  - examples から再利用できるライブラリ化
- テストとドキュメント
  - 合計 59 テスト
  - Rustdoc / README / CONTRIBUTING / CHANGELOG 整備
- `no_std` 準備の初期対応
  - `hal-api` は `no_std`
  - `core-app` は `no_std`
  - `platform-pc-sim` のみが `std` 依存

### 残っている主要テーマ

- 統合テストの拡張と回帰観点の整理
- `no_std` を CI で継続的に検証する仕組み
- `platform-esp32` の最小スケルトン追加
- HAL の設計判断を ESP32 実機制約に寄せていくこと

---

## 開発方針

### 1. 品質強化を最優先にする

直近 2〜4 週間は、新しい周辺機能を広げるよりも、
既存の `GPIO` / `I2C` / `App::tick()` / PC シミュレータの品質を上げる。

- 小さな PR 単位で進める
- 正常系、異常系、境界条件、長時間実行の回帰を増やす
- examples は実行可能サンプルとして保守し、実装の重複を減らす

### 2. HAL は ESP32 実機適合を基準に育てる

設計判断は PC シミュレータ都合ではなく、将来の `esp-hal` への適合性を優先する。

- 当面は `GPIO` / `I2C` のみを対象にする
- `SPI` / `ADC` / `Timer` は必要性が明確になるまで追加しない
- WiFi / Bluetooth は対象外
- platform 固有エラーは `hal-api` のエラー型に変換する方針を維持する

### 3. `no_std` と実機対応は段階導入する

`no_std` と ESP32 対応は、品質強化の後に小さなステップで進める。

- `hal-api` / `core-app` の `no_std` 成立を維持する
- ホスト依存処理は `platform-pc-sim` に閉じ込める
- `platform-esp32` は GPIO / I2C の最小構成で始める

---

## 直近の実装順序

### フェーズ A: 品質強化

- cross-crate の統合テストを追加する
- examples と PC シミュレータのモック HAL を共通化する
- CI で維持したい回帰観点を明文化する

### フェーズ B: `no_std` 検証の継続化

- `no_std` 向け target を用いた `cargo check` を CI に追加する
- `hal-api` / `core-app` に `std` 依存が混入しないことを検証する

### フェーズ C: ESP32 準備

- `crates/platform-esp32` を追加する
- GPIO と I2C のラッパーの骨組みを作る
- LED 制御と単純な I2C 読み取りを最初の成功条件にする

---

## 受け入れ基準

- 品質強化フェーズ
  - `cargo test --workspace` が通る
  - 既存 API の挙動がテストで固定される
  - examples の実装重複が減る
- `no_std` フェーズ
  - `hal-api` / `core-app` が `std` なしでビルド可能
  - `platform-pc-sim` のみがホスト依存を持つ
- ESP32 準備フェーズ
  - `platform-esp32` の最小クレートが workspace に追加される
  - GPIO / I2C の最低限の実装方針がコードで表現される
