# mcu-hal-sim-rs

[![CI](https://github.com/1222-takeshi/mcu-hal-sim-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/1222-takeshi/mcu-hal-sim-rs/actions/workflows/ci.yml)

マイコン向けRustアプリケーションを、**ハードウェア抽象化層（HAL）**を通じてプラットフォーム非依存に記述し、PC上のシミュレータで動作確認できるプロジェクトです。

## ✨ 特徴

- **🎯 プラットフォーム非依存**: HAL traitを使用し、同じアプリケーションコードを複数のプラットフォームで実行
- **💻 PCシミュレータ**: 実機なしで開発・デバッグが可能
- **🧪 テスト駆動開発**: 71個のテストでコードの品質を保証
- **🔧 CI/CD自動化**: GitHub Actionsで自動ビルド・テスト・Lint
- **📦 `no_std` 準備**: `hal-api` と `core-app` はホスト依存を持たない構成
- **⚙️ original ESP32 準備**: `platform-esp32` は `embedded-hal` v1.0 経由で実機HALを受けられる構成
- **🧰 実機雛形あり**: `firmware/original-esp32-bringup` から LED only / real I2C の両方を試せる
- **🚀 将来の拡張性**: ESP32、Arduino Nano、Raspberry Pi Picoなどへの対応を予定

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
│ PC Simulator │  │ ESP32 (original 準備中) │
│ platform-    │  │ platform-     │
│ pc-sim       │  │ esp32         │
│ - MockPin    │  │ - Esp32Pin    │
│ - MockI2c    │  │ - Esp32I2c    │
└──────────────┘  └───────────────────────┘
```

## 🚀 クイックスタート

### 前提条件

- Rust 1.70以降（[rustup](https://rustup.rs/)でインストール）

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

### 実行

```bash
# PCシミュレータを実行
cargo run -p platform-pc-sim
```

**期待される出力:**
```
=== PC Simulator Started ===
[GPIO] Pin 13 set HIGH
[GPIO] Pin 13 set LOW
[I2C] Read from 0x48: 4 bytes
...
```

Ctrl+Cで終了します。

### テスト

```bash
# すべてのテストを実行
cargo test --all

# 詳細出力
cargo test --all -- --nocapture

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
│   │   ├── error.rs      # エラー型（GpioError, I2cError）
│   │   ├── gpio.rs       # GPIO trait（OutputPin, InputPin）
│   │   ├── i2c.rs        # I2C trait（I2cBus）
│   │   └── lib.rs
│   │
│   ├── core-app/         # アプリケーションロジック
│   │   └── lib.rs        # App<PIN, I2C>構造体
│   │                     #   - 100 tickごとにLED点滅
│   │                     #   - 500 tickごとにI2C読み取り
│   │
│   ├── platform-pc-sim/  # PCシミュレータ
│       ├── lib.rs        # モックHAL公開
│       ├── main.rs       # エントリポイント（10ms tickループ）
│       └── mock_hal.rs   # モックHAL実装
│
│   └── platform-esp32/   # original ESP32向けアダプタ
│       ├── gpio.rs       # Esp32OutputPin / Esp32InputPin
│       ├── i2c.rs        # Esp32I2c
│       ├── lib.rs
│       └── README.md
│
├── docs/
│   └── images/                # 配線図 / bring-up フロー図
│
├── .github/
│   └── workflows/
│       └── ci.yml        # CI/CD設定
│
├── .cargo/
│   └── config.toml       # original ESP32向け cargo alias / runner
│
├── firmware/
│   └── original-esp32-bringup/
│       ├── .cargo/config.toml  # xtensa target / espflash runner
│       ├── src/main.rs         # LED only / real I2C bring-up
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
| **hal-api** | HAL trait定義（`OutputPin`, `I2cBus`など） | なし |
| **core-app** | プラットフォーム非依存のアプリケーションロジック | `hal-api` |
| **platform-pc-sim** | PCシミュレータ実装（モックHAL） | `hal-api`, `core-app` |
| **platform-esp32** | original ESP32向け `embedded-hal` アダプタ | `hal-api`, `embedded-hal` |

### 実機用テンプレート

| ディレクトリ | 説明 | 依存関係 |
|-------------|------|---------|
| **firmware/original-esp32-bringup** | original ESP32 向け bring-up 雛形 | `core-app`, `platform-esp32`, `esp-hal` |

## 🧪 テスト

このプロジェクトはテスト駆動開発（TDD）で構築されています。

| クレート | テストタイプ | テスト数 |
|---------|------------|---------|
| hal-api | ドキュメントテスト | 17個 |
| core-app | ユニット + doc test | 25個 |
| platform-pc-sim | ユニット + 統合 + doc test | 17個 |
| platform-esp32 | ユニット + 統合テスト | 12個 |
| **合計** | | **71個** |

## 🛠️ 開発

### TDD原則

このプロジェクトは以下のTDDサイクルに従います:

1. **🔴 Red**: 失敗するテストを書く
2. **🟢 Green**: テストを通すための最小限の実装
3. **🔵 Refactor**: コードを改善

詳細は `/home/takeshi_miura/workspace/CLAUDE.md` を参照してください。

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

## 📅 ロードマップ

- [x] **Week 1**: PCシミュレータの完成
- [x] **Week 2**: テスト基盤の整備（57テスト）
- [x] **Week 3**: CI/CD環境の構築
- [x] **Week 4**: ドキュメント充実
- [ ] **Week 5**: 統合テスト・カバレッジ向上（進行中）
- [ ] **Week 6**: no_std対応・ESP32準備
- [ ] **Week 7-8**: ESP32実機対応（オプション）

詳細は [CHANGELOG.md](./CHANGELOG.md) と [開発計画](https://github.com/1222-takeshi/mcu-hal-sim-rs/blob/main/.claude/plans/hazy-drifting-frost.md) を参照してください。

## 📄 ライセンス

このプロジェクトはMITライセンスの下で公開されています。

## 🙏 謝辞

このプロジェクトは、組み込みRustコミュニティの素晴らしい取り組み（特に[embedded-hal](https://github.com/rust-embedded/embedded-hal)）にインスパイアされています。
