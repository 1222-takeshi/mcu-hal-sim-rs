#!/usr/bin/env bash
#
# dev-loop.sh — シミュレータ / ESP32 フラッシュ / クロスビルドチェック
#
# 使用方法:
#   ./scripts/dev-loop.sh sim        PCシミュレータを起動
#   ./scripts/dev-loop.sh check      全CIチェック + ESP32-C3クロスビルドを実行
#   ./scripts/dev-loop.sh flash      ESP32実機へフラッシュ（espflash が必要）
#   ./scripts/dev-loop.sh monitor    フラッシュ後にシリアルモニタを開く
#
# 依存ツール:
#   sim/check: Rust stable toolchain + riscv32imc-unknown-none-elf target
#   flash/monitor: espflash (cargo install espflash)
#
# 環境変数:
#   ESP32_PORT      シリアルポート (例: /dev/cu.usbserial-0001, /dev/ttyUSB0)
#   FIRMWARE_CRATE  フラッシュ対象クレート名（[[bin]] を持つクレートが必要）
#                   デフォルト: platform-esp32 (ライブラリのみ — 実際のファームウェアに変更してください)
#
# シリアルポートの例:
#   macOS/Linux: /dev/cu.usbserial-* または /dev/ttyUSB0
#   WSL2 + Windows: espflash.exe をフルパスで指定する代替経路を使用

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

ESP32C3_TARGET="riscv32imc-unknown-none-elf"
FIRMWARE_CRATE="${FIRMWARE_CRATE:-platform-esp32}"
DASHBOARD_CRATE="platform-pc-sim"
SERIAL_PORT="${ESP32_PORT:-}"

log_step() { echo -e "${BLUE}==>${NC} $1"; }
log_ok()   { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC}  $1"; }
log_err()  { echo -e "${RED}✗${NC}  $1"; }

usage() {
  echo "Usage: $0 {sim|check|flash|monitor}"
  echo ""
  echo "  sim      PCシミュレータを起動 (http://127.0.0.1:7878)"
  echo "  check    CIチェック + ESP32-C3クロスビルドを実行"
  echo "  flash    ESP32実機へフラッシュ"
  echo "  monitor  フラッシュ後にシリアルモニタを開く"
  echo ""
  echo "環境変数:"
  echo "  ESP32_PORT    シリアルポート (例: /dev/cu.usbserial-0001, /dev/ttyUSB0)"
  exit 1
}

cmd_sim() {
  log_step "PCシミュレータを起動します..."
  log_step "ダッシュボード: http://127.0.0.1:7878"
  echo ""
  cargo run -p "$DASHBOARD_CRATE" --bin device-dashboard-web
}

cmd_check() {
  log_step "1/5 テストを実行..."
  cargo test --workspace --all-targets
  log_ok "テスト完了"

  log_step "2/5 フォーマットチェック..."
  cargo fmt --all -- --check
  log_ok "フォーマット OK"

  log_step "3/5 Clippy..."
  cargo clippy --workspace --all-targets -- -D warnings
  log_ok "Clippy OK"

  log_step "4/5 no_std チェック (thumbv6m-none-eabi)..."
  for crate in hal-api core-app reference-drivers platform-esp32 platform-avr; do
    cargo check -p "$crate" --lib --target thumbv6m-none-eabi
  done
  log_ok "no_std OK"

  log_step "5/5 ESP32-C3 クロスビルドチェック (${ESP32C3_TARGET})..."
  if ! rustup target list --installed | grep -q "$ESP32C3_TARGET"; then
    log_warn "ターゲット ${ESP32C3_TARGET} が未インストールです。追加します..."
    rustup target add "$ESP32C3_TARGET"
  fi
  for crate in hal-api core-app reference-drivers platform-esp32; do
    cargo check -p "$crate" --lib --target "$ESP32C3_TARGET"
    log_ok "  $crate: OK"
  done

  echo ""
  log_ok "全チェック完了！PR 作成準備 OK"
}

cmd_flash() {
  if ! command -v espflash &>/dev/null; then
    log_err "espflash が見つかりません。インストールしてください:"
    echo "  cargo install espflash"
    exit 1
  fi

  if [[ -z "$SERIAL_PORT" ]]; then
    log_warn "ESP32_PORT が未設定です。自動検出を試みます..."
    DETECTED=$(ls /dev/cu.usbserial-* /dev/ttyUSB* 2>/dev/null | head -1 || true)
    if [[ -z "$DETECTED" ]]; then
      log_err "シリアルポートが見つかりません。"
      echo "  macOS/Linux: ESP32_PORT=/dev/cu.usbserial-XXXX $0 flash"
      echo "  WSL2:        espflash.exe を使用してください"
      exit 1
    fi
    SERIAL_PORT="$DETECTED"
    log_step "検出: $SERIAL_PORT"
  fi

  log_step "ESP32-C3 向けリリースビルド..."
  BINARY_PATH="target/${ESP32C3_TARGET}/release/${FIRMWARE_CRATE}"
  if ! cargo build -p "$FIRMWARE_CRATE" --release --target "$ESP32C3_TARGET"; then
    log_err "ビルドに失敗しました。'${FIRMWARE_CRATE}' に [[bin]] エントリーが必要です。"
    echo "  例: FIRMWARE_CRATE=your-esp32c3-firmware $0 flash"
    exit 1
  fi
  if [[ ! -f "$BINARY_PATH" ]]; then
    log_err "バイナリが見つかりません: ${BINARY_PATH}"
    echo "  '${FIRMWARE_CRATE}' は [[bin]] を持つクレートである必要があります。"
    echo "  例: FIRMWARE_CRATE=your-esp32c3-firmware $0 flash"
    exit 1
  fi

  log_step "フラッシュ書き込み: $SERIAL_PORT"
  espflash flash \
    --port "$SERIAL_PORT" \
    "$BINARY_PATH"
  log_ok "フラッシュ完了"
}

cmd_monitor() {
  if ! command -v espflash &>/dev/null; then
    log_err "espflash が見つかりません: cargo install espflash"
    exit 1
  fi

  if [[ -z "$SERIAL_PORT" ]]; then
    DETECTED=$(ls /dev/cu.usbserial-* /dev/ttyUSB* 2>/dev/null | head -1 || true)
    SERIAL_PORT="${DETECTED:-}"
    [[ -n "$SERIAL_PORT" ]] && log_step "検出: $SERIAL_PORT"
  fi

  if [[ -z "$SERIAL_PORT" ]]; then
    log_err "ESP32_PORT が未設定です。"
    echo "  ESP32_PORT=/dev/cu.usbserial-XXXX $0 monitor"
    exit 1
  fi

  BINARY_PATH="target/${ESP32C3_TARGET}/release/${FIRMWARE_CRATE}"
  log_step "フラッシュ + モニタ: $SERIAL_PORT"
  if ! cargo build -p "$FIRMWARE_CRATE" --release --target "$ESP32C3_TARGET"; then
    log_err "ビルドに失敗しました。FIRMWARE_CRATE=${FIRMWARE_CRATE}"
    echo "  例: FIRMWARE_CRATE=your-esp32c3-firmware $0 monitor"
    exit 1
  fi
  if [[ ! -f "$BINARY_PATH" ]]; then
    log_err "バイナリが見つかりません: ${BINARY_PATH}"
    echo "  例: FIRMWARE_CRATE=your-esp32c3-firmware $0 monitor"
    exit 1
  fi
  espflash flash \
    --port "$SERIAL_PORT" \
    --monitor \
    "$BINARY_PATH"
  log_ok "モニタ終了"
}

case "${1:-}" in
  sim)     cmd_sim ;;
  check)   cmd_check ;;
  flash)   cmd_flash ;;
  monitor) cmd_monitor ;;
  *)       usage ;;
esac
