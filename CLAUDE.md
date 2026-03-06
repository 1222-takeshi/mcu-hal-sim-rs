# CLAUDE.md - mcu-hal-sim-rs

このファイルは、`mcu-hal-sim-rs` プロジェクト固有のガイドラインを提供します。

**共通の開発方針**（TDD、Git運用、PR作成ルールなど）は `/home/takeshi_miura/workspace/CLAUDE.md` を参照してください。

---

## プロジェクト概要

`mcu-hal-sim-rs`は、ESP32/Arduino Nano/Raspberry Pi Pico等のマイコン向けRustアプリケーションを、MCU非依存のHAL trait経由で記述し、PC上のシミュレータで動作確認できるようにするプロジェクトです。

### 開発目標
- ✅ **Phase 1**: PCシミュレータの完成（hal-api、core-app、platform-pc-sim）
- ✅ **Phase 2**: テスト基盤の整備（59テスト）
- ✅ **Phase 3**: CI/CD環境の構築
- 🚧 **Phase 4**: `no_std` 対応とESP32実機準備
  - `hal-api` と `core-app` は `no_std` 対応済み
  - 次は `platform-esp32` の骨組みと `no_std` 検証のCI化

---

## プロジェクト構成

```
mcu-hal-sim-rs/
├── crates/
│   ├── hal-api/          # HAL trait定義（GPIO、I2C等）
│   │   ├── error.rs      # GpioError、I2cError
│   │   ├── gpio.rs       # OutputPin、InputPin trait
│   │   ├── i2c.rs        # I2cBus trait
│   │   └── lib.rs        # モジュールルート
│   │
│   ├── core-app/         # アプリケーションロジック（プラットフォーム非依存）
│   │   └── lib.rs        # App<PIN, I2C>構造体
│   │                     # - 100 tickごとのLED点滅
│   │                     # - 500 tickごとのI2C読み取り
│   │
│   ├── platform-pc-sim/  # PCシミュレータ実装
│   │   ├── lib.rs        # モックHALの公開
│   │   ├── main.rs       # 10ms tickループ
│   │   └── mock_hal.rs   # MockPin、MockI2c実装
│   │
│   └── platform-esp32/   # ESP32実装（Week 7-8で実装予定）
│       └── (未実装)
│
├── Cargo.toml            # ワークスペース設定（resolver = "2"）
├── .gitignore            # Cargo.lockを含む
└── CLAUDE.md             # このファイル
```

### クレートの依存関係

```
platform-pc-sim  ─┐
                  ├─→ core-app ─→ hal-api
platform-esp32 ───┘       ↑          ↑
                           │          │
                      (App型)    (trait定義)
```

---

## テスト構成（現状）

| クレート | テストタイプ | テスト数 | PR |
|---------|------------|---------|-----|
| hal-api | ドキュメントテスト | 17個 | #21 |
| core-app | ユニット + doc test | 25個 | #22 他 |
| platform-pc-sim | ユニット + 統合 + doc test | 17個 | #23 他 |
| **合計** | | **59個** | |

### テスト実行コマンド

```bash
# すべてのテスト（最も一般的）
cargo test --all

# 特定のクレートのみ
cargo test -p hal-api
cargo test -p core-app
cargo test -p platform-pc-sim

# ドキュメントテストのみ
cargo test --doc -p hal-api

# 詳細出力（print!デバッグ時）
cargo test -- --nocapture

# 特定のテスト名で絞り込み
cargo test test_led_toggles
```

### テスト配置ルール

**hal-api**: ドキュメントテスト（公開APIの使用例）
```rust
/// GPIO出力ピンを制御するtrait
///
/// # Examples
///
/// ```
/// use hal_api::gpio::OutputPin;
/// // 実行可能なサンプルコード
/// ```
pub trait OutputPin { ... }
```

**core-app**: ユニットテスト（ビジネスロジックの検証）
```rust
// lib.rsの末尾
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_led_toggles_every_100_ticks() { ... }
}
```

**platform-pc-sim**: ユニットテスト（モックHALの動作確認）
```rust
// mock_hal.rsの末尾
#[cfg(test)]
mod tests {
    #[test]
    fn test_mock_pin_set_high() { ... }
}
```

---

## Rust固有のコーディング規約

### 1. エラーハンドリング

```rust
// ✅ Good: Result型と?演算子
pub fn tick(&mut self) -> Result<(), AppError> {
    self.pin.set(self.led_state)?;
    self.i2c.read(0x48, &mut buffer)?;
    Ok(())
}

// ❌ Bad: unwrap()の使用（テスト以外）
pub fn tick(&mut self) {
    self.pin.set(self.led_state).unwrap();  // 避ける
}
```

### 2. エラー型の設計

```rust
// AppErrorは具体的なHALエラーをラップ
#[derive(Debug)]
pub enum AppError {
    Gpio(GpioError),
    I2c(I2cError),
}

// From traitで?演算子が使える
impl From<GpioError> for AppError {
    fn from(err: GpioError) -> Self {
        AppError::Gpio(err)
    }
}
```

### 3. ジェネリックなHAL設計

```rust
// HAL traitに依存、具体的な実装には依存しない
pub struct App<PIN, I2C>
where
    PIN: OutputPin<Error = GpioError>,
    I2C: I2cBus<Error = I2cError>,
{
    pin: PIN,
    i2c: I2C,
    // ...
}
```

### 4. テスト用ヘルパー

```rust
// #[cfg(test)]で本番ビルドから除外
#[cfg(test)]
pub fn tick_count(&self) -> u32 {
    self.tick_count
}
```

---

## ビルドとリリース

### ローカルビルド

```bash
# 開発ビルド
cargo build

# リリースビルド（最適化）
cargo build --release

# 特定のクレートのみ
cargo build -p platform-pc-sim

# フォーマットチェック
cargo fmt -- --check

# Clippy（Linter）
cargo clippy -- -D warnings
```

### 実行

```bash
# PCシミュレータを実行
cargo run -p platform-pc-sim

# リリースビルドで実行
cargo run -p platform-pc-sim --release
```

---

## CI/CD（Week 3で実装済み）

### GitHub Actions設定

`.github/workflows/ci.yml` で以下を自動化:

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - cargo test --all --verbose

  build:
    runs-on: ubuntu-latest
    steps:
      - cargo build --all --release --verbose

  fmt:
    runs-on: ubuntu-latest
    steps:
      - cargo fmt --all -- --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - cargo clippy --all --all-targets -- -D warnings
```

### CI検証ベストプラクティス

#### ローカル検証（PRを作成する前）

```bash
# すべてのCIチェックをローカルで実行
./scripts/ci-local.sh

# または手動で実行
cargo test --all --verbose
cargo build --all --release --verbose
cargo fmt --all -- --check
cargo clippy --all --all-targets -- -D warnings
```

#### CI失敗時のデバッグ

```bash
# 最新のワークフロー状態を確認
gh run list --limit 1

# 特定のワークフローの詳細ログを取得
gh run view <run-id> --log-failed

# Clippyエラーの詳細を確認
gh run view <run-id> --log-failed 2>&1 | grep -A 20 "clippy"
```

#### よくあるCI失敗パターンと対処法

| エラー | 原因 | 対処法 |
|--------|------|--------|
| `bool_assert_comparison` | `assert_eq!(bool, true/false)` | `assert!(bool)` または `assert!(!bool)` に変更 |
| `manual_is_multiple_of` | `x % n == 0` | `#[allow(clippy::manual_is_multiple_of)]` を追加（unstable機能） |
| Formatエラー | 末尾の改行、複数の空行 | `cargo fmt --all` で自動修正 |
| `dead_code` warning | 未使用のフィールド/関数 | `#[allow(dead_code)]` を追加またはコードを削除 |

### rustfmt設定

`rustfmt.toml` の設定:

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_field_init_shorthand = true
use_try_shorthand = true
```

**注意**: unstable機能（`imports_granularity`など）はnightly必須のため使用しない

---

## no_std対応（Week 6予定）

### 現在の状況
- `hal-api`、`core-app`: `std`に依存
- `platform-pc-sim`: `std`必須（シミュレータ）

### 将来の対応方針

```rust
// hal-api/lib.rs、core-app/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;
```

```toml
# Cargo.toml
[features]
default = ["std"]
std = []
```

---

## ESP32開発（Week 7-8予定）

### 必要なツール

```bash
# espup（ESP32 Rustツールチェーン）
cargo install espup
espup install

# espflash（書き込みツール）
cargo install espflash
```

### ESP32向けビルド・書き込み

```bash
# ビルド
cargo build -p platform-esp32

# 実機書き込み・モニタ
cargo espflash flash -p platform-esp32 --monitor
```

### ESP32実装の構成

```
platform-esp32/
├── Cargo.toml
├── .cargo/config.toml
├── rust-toolchain.toml
└── src/
    ├── main.rs
    ├── esp32_gpio.rs  # Esp32OutputPin実装
    └── esp32_i2c.rs   # Esp32I2c実装
```

---

## トラブルシューティング

### ビルドエラー時

```bash
# 依存関係を更新
cargo update

# クリーンビルド
cargo clean && cargo build
```

### テスト失敗時

```bash
# 特定のテストのみ実行（詳細出力）
cargo test test_name -- --nocapture

# ログレベルを上げる
RUST_LOG=debug cargo test
```

### Cargo.lock関連

- このプロジェクトでは`.gitignore`にCargo.lockを含む
- 理由: ライブラリプロジェクト（hal-api、core-app）がメイン
- CIでは常に最新の依存関係でテスト

---

## Examples作成ガイドライン（Week 4で確立）

### Examplesの配置

Examplesはワークスペースルートの`examples/`ディレクトリに配置:

```
mcu-hal-sim-rs/
├── examples/
│   ├── basic_blink.rs      # 基本的な使用例
│   ├── i2c_read.rs          # I2C通信の例
│   └── custom_timing.rs     # 高度な例
├── Cargo.toml               # [package]セクション必須
└── crates/
```

### ワークスペースルートのCargo.toml設定

Examplesを認識させるため、ワークスペースルートに`[package]`セクションを追加:

```toml
[workspace]
resolver = "2"
members = [
    "crates/hal-api",
    "crates/core-app",
    "crates/platform-pc-sim",
]

[package]
name = "mcu-hal-sim-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
hal-api = { path = "crates/hal-api" }
core-app = { path = "crates/core-app" }
```

### Exampleのテンプレート

```rust
//! # Example Title
//!
//! 簡潔な説明（1-2文）
//!
//! ## 実行方法
//!
//! ```bash
//! cargo run --example example_name
//! ```
//!
//! ## 期待される出力
//!
//! ```text
//! 出力例
//! ```

use core_app::App;
use std::thread;
use std::time::Duration;

// モックHALの定義（各exampleで独自に定義）
mod mock_hal {
    // MockPin、MockI2cの実装
}

use mock_hal::{MockI2c, MockPin};

fn main() {
    println!("=== Example Title ===");
    // 実装
}
```

### Exampleの実行とビルド

```bash
# 単一のexampleを実行
cargo run --example basic_blink

# 全examplesをビルド
cargo build --examples

# 特定のexampleをビルド
cargo build --example i2c_read
```

### 注意点

- 各exampleは独立して実行可能にする
- コメントで初心者にも理解しやすく説明
- `#[allow(dead_code)]`で未使用警告を抑制（必要に応じて）

---

## 開発ロードマップ

| Week | フェーズ | 内容 | 状態 |
|------|---------|------|------|
| 1 | Phase 1完成 | Issue #13実装 | ✅ 完了 |
| 2 | テスト基盤 | 57個のテスト追加 | ✅ 完了 |
| 3 | CI/CD | GitHub Actions整備 | ✅ 完了 |
| 4 | ドキュメント | README、examples | 🚧 進行中 |
| 5 | 統合テスト | カバレッジ80%+ | 📅 予定 |
| 6 | no_std対応 | ESP32準備 | 📅 予定 |
| 7-8 | ESP32実装 | 実機動作確認 | 📅 オプション |

---

## 参考資料

### Rust関連
- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [embedded-hal traits](https://docs.rs/embedded-hal/latest/embedded_hal/)

### ESP32関連
- [esp-rs Book](https://esp-rs.github.io/book/)
- [espflash Documentation](https://github.com/esp-rs/espflash)

---

## 重要な原則

このプロジェクトでは **TDD（テスト駆動開発）** が必須です：

🔴 **Red**: テストを先に書く → 失敗を確認
🟢 **Green**: 最小限の実装 → テスト成功
🔵 **Refactor**: コード改善 → テスト維持

詳細は `/home/takeshi_miura/workspace/CLAUDE.md` を参照してください。
