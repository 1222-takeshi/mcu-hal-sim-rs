# Contributing to mcu-hal-sim-rs

このプロジェクトへの貢献に興味を持っていただき、ありがとうございます！

このガイドでは、開発環境のセットアップからPR作成までの手順を説明します。

---

## 📋 目次

- [開発環境のセットアップ](#開発環境のセットアップ)
- [開発ワークフロー](#開発ワークフロー)
- [プルリクエストの作成](#プルリクエストの作成)
- [コーディング規約](#コーディング規約)
- [テスト](#テスト)
- [レビュープロセス](#レビュープロセス)
- [質問・ヘルプ](#質問ヘルプ)

---

## 開発環境のセットアップ

### 必要なツール

- **Rust**: 1.70以降（安定版推奨）
- **Git**: バージョン管理
- **GitHub CLI** (推奨): `gh` コマンドでPR作成が簡単に

### Rustのインストール

```bash
# Rustがインストールされていない場合
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# バージョン確認
rustc --version
cargo --version
```

### リポジトリのクローンとビルド

```bash
# リポジトリをクローン
git clone https://github.com/1222-takeshi/mcu-hal-sim-rs.git
cd mcu-hal-sim-rs

# すべてのクレートをビルド
cargo build --all

# すべてのテストを実行
cargo test --all

# PCシミュレータを実行
cargo run -p platform-pc-sim
```

### 開発に便利なツール

```bash
# rustfmt（コードフォーマッター）
rustup component add rustfmt

# clippy（Linter）
rustup component add clippy

# GitHub CLI（PR作成用）
# macOS: brew install gh
# Ubuntu: sudo apt install gh
```

---

## 開発ワークフロー

### 🔴🟢🔵 TDD原則（必須）

このプロジェクトは **TDD（テスト駆動開発）** を採用しています。すべての機能開発はこのサイクルに従ってください：

```
🔴 Red → 🟢 Green → 🔵 Refactor
```

#### 1. 🔴 Red（失敗するテストを書く）

**最初にテストを書く** - これが最も重要です。

```rust
#[test]
fn test_led_toggles_every_100_ticks() {
    let mut app = App::new(mock_pin, mock_i2c);

    for _ in 0..100 {
        app.tick().unwrap();
    }

    assert_eq!(mock_pin.state(), true);  // ❌ 失敗（未実装）
}
```

```bash
$ cargo test
# test_led_toggles_every_100_ticks ... FAILED
```

#### 2. 🟢 Green（テストを通す最小限の実装）

テストを通すための最小限のコードを書く。

```rust
pub fn tick(&mut self) {
    self.tick_count += 1;

    if self.tick_count % 100 == 0 {
        self.led_state = !self.led_state;
        self.pin.set(self.led_state).unwrap();
    }
}
```

```bash
$ cargo test
# test_led_toggles_every_100_ticks ... ok ✅
```

#### 3. 🔵 Refactor（改善）

テストを維持しながらコードを改善。

```rust
pub fn tick(&mut self) -> Result<(), AppError> {
    self.tick_count += 1;

    if self.tick_count % 100 == 0 {
        self.led_state = !self.led_state;
        self.pin.set(self.led_state)?;  // エラーハンドリング改善
    }

    Ok(())
}
```

```bash
$ cargo test
# すべてのテスト成功 ✅
```

### 開発の流れ

1. **Issueを確認**（または新規作成）
2. **ブランチを作成**: `git checkout -b feat/issue-N-description`
3. **🔴 Red**: テストを先に書く
4. **🟢 Green**: 実装してテストを通す
5. **🔵 Refactor**: コードを改善
6. **PRを作成**
7. **レビューを受ける**
8. **マージ**

---

## プルリクエストの作成

### ブランチ命名規則

目的に応じてプレフィックスを使用してください：

| プレフィックス | 用途 | 例 |
|--------------|------|-----|
| `feat/` | 新機能の追加 | `feat/issue-13-app-logic` |
| `fix/` | バグ修正 | `fix/issue-25-gpio-error` |
| `docs/` | ドキュメント | `docs/issue-33-rustdoc` |
| `test/` | テスト追加・修正 | `test/add-i2c-tests` |
| `refactor/` | リファクタリング | `refactor/simplify-error-handling` |
| `chore/` | ビルド・CI関連 | `chore/update-ci-workflow` |

### PR作成前のチェックリスト

PRを作成する前に、以下をローカルで確認してください：

```bash
# すべてのCIチェックをローカルで実行
./scripts/ci-local.sh

# または手動で実行
cargo test --all --verbose
cargo build --all --release --verbose
cargo fmt --all -- --check
cargo clippy --all --all-targets -- -D warnings
```

**必須チェック**:
- [ ] すべてのテストが成功（`cargo test --all`）
- [ ] フォーマットが正しい（`cargo fmt --all -- --check`）
- [ ] Clippyで警告なし（`cargo clippy --all -- -D warnings`）
- [ ] 新しい機能にテストを追加
- [ ] ドキュメント（Rustdoc）を追加・更新
- [ ] CLAUDE.mdの受け入れ基準を満たす

### コミットメッセージ規約

コミットメッセージは以下の形式に従ってください：

```
<type>: <subject>

<body>

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**Type（種類）**:
- `feat`: 新機能の追加
- `fix`: バグ修正
- `docs`: ドキュメント更新
- `test`: テスト追加・修正
- `refactor`: リファクタリング
- `chore`: ビルドプロセスや補助ツールの変更

**例**:
```
feat: implement LED blinking logic

- Add tick_count field to App
- Implement LED toggle every 100 ticks
- Add error handling with AppError

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### PRタイトルとボディ

**PRタイトル**:
- 簡潔に（70文字以内）
- コミットメッセージと同じ形式

**PRボディ**:
```markdown
## Summary
- 変更内容の要約（1-3箇条書き）

## Test plan
- [x] `cargo test --all` 成功
- [x] `cargo clippy --all` 警告なし
- [x] 新しいテストを追加

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Closes #NN
```

---

## コーディング規約

### Rust固有の規約

#### 1. フォーマット

```bash
# すべてのコードをフォーマット
cargo fmt --all

# フォーマットチェック（CIで実行）
cargo fmt --all -- --check
```

プロジェクトは `rustfmt.toml` で設定されています。

#### 2. Linting

```bash
# すべてのクレートでclippyを実行
cargo clippy --all --all-targets -- -D warnings
```

警告をエラーとして扱います（CI enforces this）。

#### 3. エラーハンドリング

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

#### 4. ジェネリックなHAL設計

```rust
// HAL traitに依存、具体的な実装には依存しない
pub struct App<PIN, I2C>
where
    PIN: OutputPin<Error = GpioError>,
    I2C: I2cBus<Error = I2cError>,
{
    pin: PIN,
    i2c: I2C,
}
```

#### 5. ドキュメントコメント

すべてのpublic APIにRustdocコメントを追加してください：

```rust
/// GPIO出力ピンを制御するtrait
///
/// # Examples
///
/// ```
/// use hal_api::gpio::OutputPin;
/// // 実行可能なサンプルコード
/// ```
pub trait OutputPin {
    // ...
}
```

---

## テスト

### テストの種類

| クレート | テストタイプ | 場所 |
|---------|------------|------|
| **hal-api** | ドキュメントテスト | 各API定義内（`///` コメント） |
| **core-app** | ユニットテスト | `lib.rs` の `#[cfg(test)] mod tests` |
| **platform-pc-sim** | ユニットテスト | `mock_hal.rs` の `#[cfg(test)] mod tests` |

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

### テストのベストプラクティス

- **独立性**: テスト間で依存関係を持たない
- **冪等性**: 何度実行しても同じ結果
- **明確性**: テスト名で何を検証しているか一目瞭然
- **高速**: 遅いテストは統合テストに分離

---

## レビュープロセス

### PRレビューの流れ

1. **PRを作成**
   - GitHub上でPRを作成
   - CIが自動実行される
2. **CIの成功を確認**
   - すべてのCIチェックが成功していることを確認
   - 失敗した場合は修正してpush
3. **メンテナーがレビュー**
   - コードの品質、テスト、ドキュメントを確認
   - 必要に応じてコメント
4. **指摘事項を修正**
   - レビューコメントに対応
   - 修正をpush（同じPRに追加される）
5. **承認後、マージ**
   - メンテナーが承認
   - メンテナーがマージ（Squash & Merge）

### レビュー観点

レビューでは以下の観点を確認します：

- **コードの品質**: 可読性、保守性、効率性
- **TDD原則の遵守**: テストファーストで開発されているか
- **テストの充実度**: カバレッジ、エッジケースの確認
- **ドキュメントの正確性**: Rustdoc、README、CLAUDE.mdの更新
- **CI/CDの成功**: すべてのCIチェックが成功しているか
- **エラーハンドリング**: 適切な`Result`型の使用

---

## CI/CD

### GitHub Actionsワークフロー

`.github/workflows/ci.yml` で以下を自動化:

- **test**: `cargo test --all --verbose`
- **build**: `cargo build --all --release --verbose`
- **fmt**: `cargo fmt --all -- --check`
- **clippy**: `cargo clippy --all --all-targets -- -D warnings`

### ローカルでのCI検証

PRを作成する前に、必ずローカルで検証してください：

```bash
# すべてのCIチェックをローカルで実行
./scripts/ci-local.sh

# または手動で実行
cargo test --all --verbose
cargo build --all --release --verbose
cargo fmt --all -- --check
cargo clippy --all --all-targets -- -D warnings
```

### よくあるCI失敗パターンと対処法

| エラー | 原因 | 対処法 |
|--------|------|--------|
| `bool_assert_comparison` | `assert_eq!(bool, true/false)` | `assert!(bool)` または `assert!(!bool)` に変更 |
| `manual_is_multiple_of` | `x % n == 0` | `#[allow(clippy::manual_is_multiple_of)]` を追加 |
| Formatエラー | 末尾の改行、複数の空行 | `cargo fmt --all` で自動修正 |
| `dead_code` warning | 未使用のフィールド/関数 | `#[allow(dead_code)]` を追加またはコードを削除 |

---

## 質問・ヘルプ

### 質問がある場合

- **GitHub Issues**: 新しいIssueを作成して質問
- **GitHub Discussions**: 議論や提案はDiscussionsで
- **README.md**: プロジェクト概要を確認

### 貢献のアイデア

以下のような貢献を歓迎します：

- 🐛 バグ修正
- ✨ 新機能の提案・実装
- 📝 ドキュメントの改善
- 🧪 テストの追加
- 🎨 コードの改善・リファクタリング
- 🔧 ビルドスクリプトやCI/CDの改善

### 良い最初の課題

以下のラベルのIssueから始めることをお勧めします：

- `good first issue`: 初心者向け
- `documentation`: ドキュメント改善
- `test`: テスト追加

---

## 参考資料

### Rust関連

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [embedded-hal traits](https://docs.rs/embedded-hal/latest/embedded_hal/)

### プロジェクト固有

- [CLAUDE.md](./CLAUDE.md): プロジェクト固有の詳細ガイドライン
- [README.md](./README.md): プロジェクト概要
- [CHANGELOG.md](./CHANGELOG.md): 変更履歴

---

## ライセンス

このプロジェクトに貢献することで、あなたのコントリビューションがプロジェクトと同じライセンス（MIT/Apache-2.0）の下で公開されることに同意したものとみなされます。

---

**ありがとうございます！** あなたの貢献がこのプロジェクトをより良いものにします 🚀
