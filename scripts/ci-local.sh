#!/usr/bin/env bash
#
# CI Local Verification Script
#
# このスクリプトは、GitHub Actionsと同じCI検証をローカルで実行します。
# PRを作成する前に実行することで、CIエラーを事前に発見できます。
#
# 使用方法:
#   ./scripts/ci-local.sh
#
# オプション:
#   --skip-test     テストをスキップ
#   --skip-build    ビルドをスキップ
#   --skip-fmt      フォーマットチェックをスキップ
#   --skip-clippy   Clippyチェックをスキップ
#   --fix           可能な問題を自動修正（fmt, clippy --fix）

set -euo pipefail

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# オプション解析
SKIP_TEST=false
SKIP_BUILD=false
SKIP_FMT=false
SKIP_CLIPPY=false
FIX_MODE=false

for arg in "$@"; do
    case $arg in
        --skip-test)
            SKIP_TEST=true
            ;;
        --skip-build)
            SKIP_BUILD=true
            ;;
        --skip-fmt)
            SKIP_FMT=true
            ;;
        --skip-clippy)
            SKIP_CLIPPY=true
            ;;
        --fix)
            FIX_MODE=true
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            echo "Usage: $0 [--skip-test] [--skip-build] [--skip-fmt] [--skip-clippy] [--fix]"
            exit 1
            ;;
    esac
done

# ヘッダー
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  CI Local Verification${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# カウンター
PASSED=0
FAILED=0

# 関数: セクションヘッダー
print_section() {
    echo -e "\n${YELLOW}[$1]${NC}"
}

# 関数: 成功メッセージ
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
    ((PASSED++))
}

# 関数: 失敗メッセージ
print_failure() {
    echo -e "${RED}✗ $1${NC}"
    ((FAILED++))
}

# 1. テスト
if [ "$SKIP_TEST" = false ]; then
    print_section "1/4 Running Tests"
    if cargo test --all --verbose; then
        print_success "All tests passed"
    else
        print_failure "Tests failed"
    fi
else
    echo -e "${YELLOW}Skipping tests${NC}"
fi

# 2. ビルド
if [ "$SKIP_BUILD" = false ]; then
    print_section "2/4 Building Release"
    if cargo build --all --release --verbose; then
        print_success "Build succeeded"
    else
        print_failure "Build failed"
    fi
else
    echo -e "${YELLOW}Skipping build${NC}"
fi

# 3. フォーマット
if [ "$SKIP_FMT" = false ]; then
    print_section "3/4 Checking Format"
    if ! command -v rustfmt &> /dev/null; then
        echo -e "${YELLOW}Warning: rustfmt not found, skipping format check${NC}"
        echo -e "${YELLOW}Install with: rustup component add rustfmt${NC}"
    else
        if [ "$FIX_MODE" = true ]; then
            echo "Auto-fixing format issues..."
            cargo fmt --all
            print_success "Format fixed"
        else
            if cargo fmt --all -- --check; then
                print_success "Format check passed"
            else
                print_failure "Format check failed (run with --fix to auto-fix)"
            fi
        fi
    fi
else
    echo -e "${YELLOW}Skipping format check${NC}"
fi

# 4. Clippy
if [ "$SKIP_CLIPPY" = false ]; then
    print_section "4/4 Running Clippy"
    if ! command -v cargo-clippy &> /dev/null && ! cargo clippy --version &> /dev/null; then
        echo -e "${YELLOW}Warning: clippy not found, skipping clippy check${NC}"
        echo -e "${YELLOW}Install with: rustup component add clippy${NC}"
    else
        if [ "$FIX_MODE" = true ]; then
            echo "Auto-fixing clippy issues..."
            if cargo clippy --all --all-targets --fix --allow-dirty --allow-staged -- -D warnings; then
                print_success "Clippy fixed"
            else
                print_failure "Clippy fix failed (some issues may require manual fixes)"
            fi
        else
            if cargo clippy --all --all-targets -- -D warnings; then
                print_success "Clippy check passed"
            else
                print_failure "Clippy check failed (run with --fix to auto-fix)"
            fi
        fi
    fi
else
    echo -e "${YELLOW}Skipping clippy${NC}"
fi

# サマリー
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All CI checks passed! Ready to push.${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some CI checks failed. Please fix the issues before pushing.${NC}"
    exit 1
fi
