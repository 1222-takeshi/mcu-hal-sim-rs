# CLAUDE.md - mcu-hal-sim-rs

このファイルは、`mcu-hal-sim-rs` プロジェクト固有のガイドラインを提供します。

**共通の開発方針**（TDD、Git運用、PR作成ルールなど）は `/home/takeshi_miura/workspace/CLAUDE.md` を参照してください。

---

## プロジェクト概要

`mcu-hal-sim-rs`は、ESP32/Arduino Nano/Raspberry Pi Pico等のマイコン向けRustアプリケーションを、MCU非依存のHAL trait経由で記述し、PC上のシミュレータで動作確認できるようにするプロジェクトです。

### 開発目標
- ✅ **Phase 1**: PCシミュレータの完成（hal-api、core-app、platform-pc-sim）
- 🚧 **Phase 2**: テスト基盤の整備（現在進行中 - Week 2）
- 📅 **Phase 3**: CI/CD環境の構築（Week 3）
- 📅 **Phase 4**: no_std対応とESP32実機対応（Week 6-8）

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

## テスト構成（Week 2で整備済み）

| クレート | テストタイプ | テスト数 | PR |
|---------|------------|---------|-----|
| hal-api | ドキュメントテスト | 17個 | #21 |
| core-app | ユニットテスト | 20個 | #22 |
| platform-pc-sim | ユニットテスト | 20個 | #23 |
| **合計** | | **57個** | |

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

## CI/CD（Week 3で実装予定）

`.github/workflows/ci.yml` で以下を自動化:

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - cargo test --all

  build:
    runs-on: ubuntu-latest
    steps:
      - cargo build --all --release

  fmt:
    runs-on: ubuntu-latest
    steps:
      - cargo fmt -- --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - cargo clippy -- -D warnings
```

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

## 開発ロードマップ

| Week | フェーズ | 内容 | 状態 |
|------|---------|------|------|
| 1 | Phase 1完成 | Issue #13実装 | ✅ 完了 |
| 2 | テスト基盤 | 57個のテスト追加 | ✅ 完了 |
| 3 | CI/CD | GitHub Actions整備 | 📅 予定 |
| 4 | ドキュメント | README、examples | 📅 予定 |
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
