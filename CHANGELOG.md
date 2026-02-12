# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- 今後追加される機能

### Changed
- 今後変更される機能

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
