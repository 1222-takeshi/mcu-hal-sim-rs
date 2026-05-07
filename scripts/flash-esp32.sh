#!/usr/bin/env bash
#
# flash-esp32.sh — ESP32 ファームウェアのワンコマンドフラッシュスクリプト
#
# 使用方法:
#   ./scripts/flash-esp32.sh <firmware-dir> [port]
#
# 引数:
#   firmware-dir  ファームウェアのディレクトリ (例: firmware/original-esp32-robot-base)
#   port          シリアルポート (省略時は自動検出)
#
# 例:
#   ./scripts/flash-esp32.sh firmware/original-esp32-robot-base
#   ./scripts/flash-esp32.sh firmware/original-esp32-climate-display /dev/ttyUSB0
#
# 依存ツール:
#   espflash  (cargo install espflash)
#   esp       xtensa ツールチェーン (espup install)
#
# シリアルポート自動検出:
#   macOS: /dev/cu.usbserial-* または /dev/cu.SLAB_USBtoUART* または /dev/cu.wchusbserial*
#   Linux: /dev/ttyUSB0, /dev/ttyUSB1, /dev/ttyACM0
#   WSL2 + Windows: 環境変数 ESP32_PORT に COM ポートを指定するか
#                   espflash.exe をフルパスで使用してください

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info()  { echo -e "${BLUE}[INFO]${NC}  $*"; }
log_ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# ── 引数チェック ───────────────────────────────────────────────────────────────
if [[ $# -lt 1 ]]; then
    echo -e "${CYAN}使用方法:${NC} $0 <firmware-dir> [port]"
    echo ""
    echo "  firmware-dir   ファームウェアのディレクトリ"
    echo "                 例: firmware/original-esp32-robot-base"
    echo "                     firmware/original-esp32-climate-display"
    echo "  port           シリアルポート（省略時は自動検出）"
    echo ""
    echo "例:"
    echo "  $0 firmware/original-esp32-robot-base"
    echo "  $0 firmware/original-esp32-climate-display /dev/ttyUSB0"
    exit 1
fi

FIRMWARE_DIR="${1}"
MANUAL_PORT="${2:-}"

# ── ファームウェアディレクトリ確認 ─────────────────────────────────────────────
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIRMWARE_PATH="${REPO_ROOT}/${FIRMWARE_DIR}"

if [[ ! -d "${FIRMWARE_PATH}" ]]; then
    log_error "ファームウェアディレクトリが見つかりません: ${FIRMWARE_PATH}"
    exit 1
fi

if [[ ! -f "${FIRMWARE_PATH}/Cargo.toml" ]]; then
    log_error "Cargo.toml が見つかりません: ${FIRMWARE_PATH}/Cargo.toml"
    exit 1
fi

# ── espflash 確認 ──────────────────────────────────────────────────────────────
if ! command -v espflash &>/dev/null; then
    log_error "espflash が見つかりません。以下でインストールしてください:"
    echo "    cargo install espflash"
    exit 1
fi
log_ok "espflash: $(espflash --version 2>&1 | head -1)"

# ── シリアルポート検出 ─────────────────────────────────────────────────────────
detect_port() {
    local port=""

    # 環境変数で指定されている場合はそれを使用
    if [[ -n "${ESP32_PORT:-}" ]]; then
        echo "${ESP32_PORT}"
        return
    fi

    # macOS: /dev/cu.usbserial-*, /dev/cu.SLAB_USBtoUART*, /dev/cu.wchusbserial*
    if [[ "$(uname)" == "Darwin" ]]; then
        for pattern in \
            "/dev/cu.usbserial-*" \
            "/dev/cu.SLAB_USBtoUART*" \
            "/dev/cu.wchusbserial*" \
            "/dev/cu.usbmodem*"; do
            # shellcheck disable=SC2086
            port="$(ls ${pattern} 2>/dev/null | head -1)"
            if [[ -n "${port}" ]]; then
                echo "${port}"
                return
            fi
        done
    fi

    # Linux: /dev/ttyUSB*, /dev/ttyACM*
    if [[ "$(uname)" == "Linux" ]]; then
        for device in /dev/ttyUSB0 /dev/ttyUSB1 /dev/ttyACM0 /dev/ttyACM1; do
            if [[ -e "${device}" ]]; then
                echo "${device}"
                return
            fi
        done
    fi

    echo ""
}

if [[ -n "${MANUAL_PORT}" ]]; then
    PORT="${MANUAL_PORT}"
    log_info "シリアルポート（指定）: ${PORT}"
else
    PORT="$(detect_port)"
    if [[ -z "${PORT}" ]]; then
        log_warn "シリアルポートを自動検出できませんでした。"
        log_warn "ESP32 を接続してから再試行するか、ポートを明示的に指定してください:"
        echo "    $0 ${FIRMWARE_DIR} /dev/cu.usbserial-XXXX   # macOS"
        echo "    $0 ${FIRMWARE_DIR} /dev/ttyUSB0              # Linux"
        echo "    ESP32_PORT=/dev/ttyUSB0 $0 ${FIRMWARE_DIR}  # 環境変数"
        exit 1
    fi
    log_ok "シリアルポート（自動検出）: ${PORT}"
fi

# ── ビルド ──────────────────────────────────────────────────────────────────────
log_info "ビルド中: ${FIRMWARE_DIR}"
cd "${FIRMWARE_PATH}"
cargo build --release
log_ok "ビルド完了"

# ── フラッシュ + モニタ ─────────────────────────────────────────────────────────
log_info "フラッシュ中: ${PORT}"
echo -e "${CYAN}------------------------------------------------------------------${NC}"
echo -e "${CYAN}  Ctrl+C でモニタを終了します${NC}"
echo -e "${CYAN}------------------------------------------------------------------${NC}"
espflash flash --port "${PORT}" --monitor --release
