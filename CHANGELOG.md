# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- `reference-drivers` を追加し、`BME280` / `LCD1602` の board 非依存 driver を `platform-esp32` から切り出した
- `platform-avr` を追加し、AVR 系 board 向けの generic GPIO / I2C adapter と host integration test を用意した
- `firmware/arduino-nano-bringup` を追加し、classic Arduino Nano 向け LED / serial / I2C scan の最小経路を用意した
- `ClimateDisplayConfig` に初回 refresh 方針を追加し、app 側の observability 向け getter を公開した
- `Bme280Config` と `Lcd1602Config` を追加し、sensor / display 差分を config struct へ閉じ込めた
- `docs/porting-and-extension-guide.md` を追加し、board / sensor 拡張時の設計ルールを整理した
- crate-level README（`hal-api`, `core-app`, `platform-pc-sim`）を追加した
- GitHub Issue Template を追加し、bug report / feature request で board / sensor 拡張情報を揃えやすくした
- `platform-pc-sim` にライブラリターゲットを追加し、`mock_hal` を examples と統合テストから再利用可能にした
- `platform-pc-sim` に cross-crate の統合テストを追加し、`core-app` と PC シミュレータ用モックHALの組み合わせを検証できるようにした
- `platform-pc-sim` に virtual I2C bus と `BME280` mock device を追加し、`platform-esp32::Bme280Sensor` を host 上で検証できるようにした
- `platform-pc-sim` に `LCD1602` mock device と `climate-dashboard-sim` を追加し、sensor / LCD / I2C / wiring view を terminal 上で確認できるようにした
- `hal-api` に distance / IMU / actuator の board 非依存 trait を追加し、`HC-SR04` / `MPU6050` / servo / DC motor / motor driver へ広げる基盤を追加した
- `platform-pc-sim` に browser 向け `device-dashboard-web` を追加し、climate / distance / IMU / servo / motor driver を 1 画面で確認できるようにした
- `reference-drivers` に `MPU6050` driver を追加し、`platform-pc-sim` に host-side `MPU6050` mock device と bridge test を追加した
- `reference-drivers` に `HC-SR04` driver を追加し、`platform-pc-sim` に pulse/echo mock device と bridge test を追加した
- CI に `hal-api` / `core-app` / `platform-esp32` の `no_std` ターゲットチェックを追加した
- `platform-esp32` クレートを追加し、GPIO / I2C 向けの最小アダプタ骨組みを導入した
- `.cargo/config.toml` に original ESP32 向け `cargo check-esp32` alias と `espflash` runner を追加した
- `crates/platform-esp32/README.md` を追加し、original ESP32 向け toolchain と最小確認手順を整理した
- `firmware/original-esp32-bringup` を追加し、LED only / real I2C の実機 bring-up 雛形を用意した
- `docs/images/original-esp32-wiring.svg` と `docs/images/original-esp32-bringup-flow.svg` を追加した

### Changed
- `platform-esp32` は board adapter に集中し、`BME280` / `LCD1602` driver は `reference-drivers` から re-export する構成になった
- `device-dashboard-web` は IMU を sequence ではなく `MPU6050` の virtual I2C mock + reference driver から読む構成になった
- `device-dashboard-web` は distance を sequence ではなく `HC-SR04` の pulse/echo mock + reference driver から読む構成になった
- `platform-pc-sim` の terminal renderer が broken pipe で panic しにくい実装になった
- `firmware/original-esp32-climate-display` が rendered frame と sensor 値を serial log へ出すようになった
- crate manifest に publish / docs.rs 前提の metadata を追加した
- `README.md` と `platform-esp32` / firmware README を、reference path と将来の board / sensor 拡張を意識した説明へ更新した
- `hal-api` と `core-app` を `no_std` 前提の構成に変更した
- `basic_blink` と `i2c_read` examples が `platform-pc-sim` のモックHALを再利用するようにした
- `PLAN.md` / `README.md` / `CLAUDE.md` を現状の実装フェーズとテスト数に合わせて更新した
- `platform-esp32` の GPIO / I2C アダプタを `embedded-hal` v1.0 互換実装へ接続する形に変更した
- `platform-esp32` の GPIO / I2C アダプタが `hal-api` のエラー型へ正規化するようになり、`core-app::App` と直接接続できるようになった

### Fixed
- 今後修正されるバグ

---

## [0.1.0] - 2026-02-12

初回リリース。マイコン向けRustアプリケーションをPCシミュレータで動作確認できる基盤を構築。

### Week 4: ドキュメント充実

#### Added
- **包括的なドキュメント**: すべてのpublic APIにRustdocコメントを追加 ([#36](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/36))
  - core-appに5個のdoc testを追加（Examples セクション）
  - モジュールレベル、構造体、メソッドすべてにドキュメント
  - `cargo doc --no-deps` 警告なし、22個のdoc test成功
- **開発者ガイド**: CONTRIBUTING.md作成 ([#37](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/37))
  - TDD原則（Red → Green → Refactor）の詳細説明
  - ブランチ命名規則、PR作成手順、コーディング規約
  - テスト実行方法、CI/CD検証手順
  - よくあるCI失敗パターンと対処法
- **変更履歴**: CHANGELOG.md作成（このファイル）
- **README.md**: プロジェクト概要とクイックスタートガイド ([#28](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/28))
  - アーキテクチャ図、クレート構成、テスト統計
  - CI/CDバッジ、ロードマップ
  - CONTRIBUTING.mdへのリンク
- **実行可能なExamples**: 3つのサンプルプログラム ([#30](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/30))
  - `basic_blink.rs`: LED点滅の基本例
  - `i2c_read.rs`: I2C通信の例
  - `custom_timing.rs`: カスタムタイミング制御
- **CI自動化スクリプト** ([#31](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/31))
  - `ci-local.sh`: ローカルで全CIチェックを実行（--fixオプション付き）
  - `ci-wait.sh`: GitHub Actions完了を自動監視
- **AI開発支援**: Claude Code Skillsを3つ作成
  - `plan-review`: OpenAI Codex CLIでIssueレビュー
  - `tdd-implement`: Codex CLIとClaude Codeの協調TDD実装
  - `code-review`: GitHub Copilot CLIでPRコードレビュー

#### Changed
- README.md: CONTRIBUTING.mdへのリンクとTDD原則を追加 ([#37](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/37))
- CLAUDE.md: CI/CD best practices、examples guidelines、AI review workflowを追加

### Week 3: CI/CD環境の構築

#### Added
- **GitHub Actions CI/CDワークフロー** ([#25](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/25))
  - 自動テスト実行（57テスト）
  - リリースビルド検証
  - `cargo fmt`によるフォーマットチェック
  - `cargo clippy`によるLintチェック（`-D warnings`）
- **rustfmt設定** ([#25](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/25))
  - `rustfmt.toml`でコードスタイル統一
  - `edition = "2021"`、`max_width = 100`、Unix改行

#### Fixed
- Clippy警告の修正: `bool_assert_comparison`、`manual_is_multiple_of` ([#25](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/25))

### Week 2: テスト基盤の整備

#### Added
- **hal-api: 17個のドキュメントテスト** ([#21](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/21))
  - `OutputPin`、`I2cBus`、`GpioError`、`I2cError`の使用例
  - 実行可能なコード例でAPIの使い方を明示
- **core-app: 20個のユニットテスト** ([#22](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/22))
  - LED点滅タイミングのテスト（100 tick周期）
  - I2C読み取りタイミングのテスト（500 tick周期）
  - エラーハンドリングのテスト（GPIO/I2Cエラー伝播）
  - エッジケースのテスト（連続動作、エラー停止）
- **platform-pc-sim: 20個のユニットテスト** ([#23](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/23))
  - `MockPin`のGPIO動作テスト
  - `MockI2c`の通信動作テスト
  - トレイト実装の検証
- **合計57個のテスト**: すべてのクレートで包括的なテストカバレッジ

### Week 1: PCシミュレータの完成

#### Added
- **HAL trait定義** ([#14](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/14))
  - `OutputPin` trait: GPIO出力ピン制御（`set_high()`、`set_low()`、`set()`）
  - `InputPin` trait: GPIO入力ピン読み取り（`is_high()`、`is_low()`）
  - `I2cBus` trait: I2C通信（`write()`、`read()`、`write_read()`）
  - `GpioError`、`I2cError`: 統一されたエラー型
- **モックHAL実装** ([#15](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/15))
  - `MockPin`: GPIO出力ピンのPC用モック実装
  - `MockI2c`: I2CバスのPC用モック実装
  - コンソール出力でハードウェア動作をシミュレート
- **アプリケーションロジック** ([#19](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/19))
  - `App<PIN, I2C>`: プラットフォーム非依存のアプリケーション構造体
  - 100 tickごとのLED点滅（1秒周期想定）
  - 500 tickごとのI2Cセンサ読み取り（5秒周期想定）
  - `AppError`: GPIO/I2Cエラーの統一的なハンドリング
- **Cargoワークスペース設定** ([#18](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/18))
  - `resolver = "2"`: Cargo 2021 edition feature resolver
  - 3クレート構成（`hal-api`、`core-app`、`platform-pc-sim`）
- **.gitignore設定** ([#20](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/20))
  - `target/`、`Cargo.lock`、IDEファイルを除外
  - ライブラリプロジェクトとしての適切な設定

#### Fixed
- Cargo.lock handling: ライブラリプロジェクトでは`.gitignore`に含める ([#20](https://github.com/1222-takeshi/mcu-hal-sim-rs/pull/20))

---

## 今後の予定

### Week 5: 統合テスト・カバレッジ向上（予定）
- 統合テストの追加
- テストカバレッジ80%以上を目指す
- パフォーマンステストの追加

### Week 6: no_std対応・ESP32準備（予定）
- `hal-api`、`core-app`の`no_std`対応
- ESP32開発環境のセットアップ
- `platform-esp32`クレートの骨組み作成

### Week 7-8: ESP32実機対応（オプション）
- ESP32向けGPIO実装（`Esp32OutputPin`）
- ESP32向けI2C実装（`Esp32I2c`）
- 実機での動作確認

---

## 貢献方法

変更履歴の記録方法については [CONTRIBUTING.md](./CONTRIBUTING.md) を参照してください。

### PRマージ時
各PRがマージされた際に、`[Unreleased]`セクションに追加：

```markdown
## [Unreleased]

### Added
- New feature description ([#PR番号](PR URL))

### Fixed
- Bug fix description ([#PR番号](PR URL))
```

### バージョンリリース時
`[Unreleased]`の内容を新しいバージョンセクションに移動：

```markdown
## [0.2.0] - YYYY-MM-DD

### Added
- (Unreleased の内容を移動)

## [Unreleased]
(空にする)
```

---

## リンク

- [GitHub Repository](https://github.com/1222-takeshi/mcu-hal-sim-rs)
- [Issues](https://github.com/1222-takeshi/mcu-hal-sim-rs/issues)
- [Pull Requests](https://github.com/1222-takeshi/mcu-hal-sim-rs/pulls)
- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
