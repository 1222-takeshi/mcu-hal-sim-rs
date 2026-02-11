# Scripts

このディレクトリには、開発を支援するスクリプトが含まれています。

## 📋 スクリプト一覧

### `ci-local.sh` - ローカルCI検証

PRを作成する前に、GitHub Actionsと同じCIチェックをローカルで実行します。

**基本的な使用方法:**

```bash
./scripts/ci-local.sh
```

**オプション:**

| オプション | 説明 |
|-----------|------|
| `--skip-test` | テストをスキップ |
| `--skip-build` | ビルドをスキップ |
| `--skip-fmt` | フォーマットチェックをスキップ |
| `--skip-clippy` | Clippyチェックをスキップ |
| `--fix` | 可能な問題を自動修正（fmt, clippy --fix） |

**使用例:**

```bash
# すべてのチェックを実行
./scripts/ci-local.sh

# 自動修正モード
./scripts/ci-local.sh --fix

# テストとビルドのみ（fmt/clippyをスキップ）
./scripts/ci-local.sh --skip-fmt --skip-clippy
```

**実行されるチェック:**

1. **Tests**: `cargo test --all --verbose`
2. **Build**: `cargo build --all --release --verbose`
3. **Format**: `cargo fmt --all -- --check`
4. **Clippy**: `cargo clippy --all --all-targets -- -D warnings`

---

### `ci-wait.sh` - CI結果監視

PRをプッシュした後、GitHub ActionsのCI完了を自動で監視し、結果を報告します。

**基本的な使用方法:**

```bash
./scripts/ci-wait.sh
```

**特定のrun-idを監視:**

```bash
./scripts/ci-wait.sh 21797882688
```

**動作:**

1. 指定されたワークフロー（またはrun-id未指定の場合は最新）を監視
2. 10秒ごとにステータスをチェック
3. 完了後、各ジョブの結果を表示
4. 失敗時は詳細ログを表示（最後の100行）

**必要な環境:**

- GitHub CLI (`gh`) がインストールされていること
- GitHubリポジトリで認証済みであること

**インストール:**

```bash
# macOS
brew install gh

# Linux (Debian/Ubuntu)
sudo apt install gh

# 認証
gh auth login
```

---

## 🔧 推奨ワークフロー

### PRを作成する前

```bash
# 1. ローカルでCIチェックを実行
./scripts/ci-local.sh

# 2. 問題がある場合は自動修正
./scripts/ci-local.sh --fix

# 3. 全てパスしたらコミット・プッシュ
git add -A
git commit -m "fix: resolve CI issues"
git push origin feature-branch
```

### PRをプッシュした後

```bash
# CI完了を自動監視
./scripts/ci-wait.sh
```

---

## 🎯 トラブルシューティング

### `rustfmt` または `clippy` が見つからない

```bash
# rustfmtをインストール
rustup component add rustfmt

# clippyをインストール
rustup component add clippy
```

### `gh` コマンドが見つからない

GitHub CLIをインストールしてください:
- https://cli.github.com/

### CI監視がタイムアウトする

デフォルトのタイムアウトは10分です。非常に遅いワークフローの場合は、スクリプトの`MAX_WAIT`変数を調整してください。

---

## 📝 カスタマイズ

これらのスクリプトはプロジェクトのニーズに合わせてカスタマイズできます。

### `ci-local.sh` のカスタマイズ例

追加のチェックを実行したい場合:

```bash
# 5. セキュリティチェック
print_section "5/5 Running Security Audit"
if cargo audit; then
    print_success "Security audit passed"
else
    print_failure "Security issues found"
fi
```

### `ci-wait.sh` のカスタマイズ例

異なるワークフローを監視したい場合:

```bash
# デフォルトのワークフロー名を変更
RUN_ID=$(gh run list --workflow=custom-workflow.yml --limit 1 --json databaseId --jq '.[0].databaseId')
```
