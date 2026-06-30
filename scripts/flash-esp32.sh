#!/usr/bin/env bash
#
# flash-esp32.sh — ESP32 ファームウェアのワンコマンドフラッシュスクリプト
#
# 使用方法:
#   ./scripts/flash-esp32.sh <firmware-dir> [port]
#   ./scripts/flash-esp32.sh <firmware-dir> --ota <IP[:PORT]>
#
# 引数:
#   firmware-dir      ファームウェアのディレクトリ (例: firmware/original-esp32-robot-base)
#   port              シリアルポート (省略時は自動検出)
#   --ota <IP[:PORT]> USB の代わりに WiFi OTA で書き込む
#                     ESP32 が original-esp32-ota-bringup で起動している必要があります。
#                     PORT 省略時は 8080 を使用します。
#
# 例:
#   ./scripts/flash-esp32.sh firmware/original-esp32-robot-base
#   ./scripts/flash-esp32.sh firmware/original-esp32-climate-display /dev/ttyUSB0
#   ./scripts/flash-esp32.sh firmware/original-esp32-climate-display --ota 192.168.1.42
#   ./scripts/flash-esp32.sh firmware/original-esp32-climate-display --ota 192.168.1.42:8080
#
# 依存ツール:
#   espflash  (cargo install espflash)           — USB フラッシュ / バイナリ生成
#   curl                                          — OTA 転送 (通常 OS 同梱)
#   esp       xtensa ツールチェーン (espup install)
#
# シリアルポート自動検出 (USB モード):
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
OTA_TARGET=""

# ── OTA モード検出 (--ota <IP[:PORT]>) ─────────────────────────────────────────
if [[ "${2:-}" == "--ota" ]]; then
    if [[ -z "${3:-}" ]]; then
        log_error "--ota には <IP> または <IP:PORT> を指定してください"
        echo "    例: $0 ${FIRMWARE_DIR} --ota 192.168.1.42"
        exit 1
    fi
    OTA_TARGET="${3}"
    MANUAL_PORT=""
fi

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

# ── ビルド ──────────────────────────────────────────────────────────────────────
log_info "ビルド中: ${FIRMWARE_DIR}"
cd "${FIRMWARE_PATH}"
cargo build --release
log_ok "ビルド完了"

# ── OTA モード ──────────────────────────────────────────────────────────────────
if [[ -n "${OTA_TARGET}" ]]; then
    # OTA_TARGET が IP のみの場合はデフォルトポートを付加する。
    if [[ "${OTA_TARGET}" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        OTA_HOST="${OTA_TARGET}"
        OTA_PORT="8080"
    else
        OTA_HOST="${OTA_TARGET%:*}"
        OTA_PORT="${OTA_TARGET##*:}"
    fi

    # espflash でアプリバイナリを抽出する。
    BINARY_PATH="/tmp/ota_firmware_$$.bin"
    log_info "OTA バイナリを抽出中…"
    if ! espflash save-image --chip esp32 "${BINARY_PATH}" 2>/dev/null; then
        # 古い espflash バージョン向けフォールバック
        BIN_PATH="$(find target -name "*.bin" -path "*/release/*" | head -1)"
        if [[ -z "${BIN_PATH}" ]]; then
            log_error "OTA バイナリが見つかりません。先に cargo build --release を実行してください。"
            exit 1
        fi
        cp "${BIN_PATH}" "${BINARY_PATH}"
    fi
    log_ok "バイナリサイズ: $(du -h "${BINARY_PATH}" | cut -f1)"

    # curl で ESP32 の OTA エンドポイントに送信する。
    OTA_URL="http://${OTA_HOST}:${OTA_PORT}/ota"
    log_info "OTA 送信中: ${OTA_URL}"
    echo -e "${CYAN}------------------------------------------------------------------${NC}"

    HTTP_STATUS=$(curl \
        --silent \
        --show-error \
        --write-out "%{http_code}" \
        --output /tmp/ota_response_$$.txt \
        --request POST \
        --header "Content-Type: application/octet-stream" \
        --data-binary "@${BINARY_PATH}" \
        --max-time 120 \
        "${OTA_URL}" || echo "000")

    OTA_RESPONSE=$(cat /tmp/ota_response_$$.txt 2>/dev/null || echo "")
    rm -f "${BINARY_PATH}" /tmp/ota_response_$$.txt

    echo -e "${CYAN}------------------------------------------------------------------${NC}"

    if [[ "${HTTP_STATUS}" == "200" ]]; then
        log_ok "OTA 書き込み成功 — ESP32 が再起動します"
        log_info "レスポンス: ${OTA_RESPONSE}"
    else
        log_error "OTA 失敗 (HTTP ${HTTP_STATUS})"
        [[ -n "${OTA_RESPONSE}" ]] && log_error "レスポンス: ${OTA_RESPONSE}"
        echo ""
        echo "確認事項:"
        echo "  1. ESP32 が original-esp32-ota-bringup で起動しているか"
        echo "  2. IP アドレスが正しいか  (シリアルログで確認: espflash monitor)"
        echo "  3. WiFi に接続できているか"
        echo "  4. ポート ${OTA_PORT} がファイアウォールでブロックされていないか"
        exit 1
    fi
    exit 0
fi

# ── USB シリアルモード ──────────────────────────────────────────────────────────
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
        echo ""
        echo "OTA で書き込む場合:"
        echo "    $0 ${FIRMWARE_DIR} --ota <ESP32のIPアドレス>"
        exit 1
    fi
    log_ok "シリアルポート（自動検出）: ${PORT}"
fi

# ── フラッシュ + モニタ ─────────────────────────────────────────────────────────
log_info "フラッシュ中: ${PORT}"
echo -e "${CYAN}------------------------------------------------------------------${NC}"
echo -e "${CYAN}  Ctrl+C でモニタを終了します${NC}"
echo -e "${CYAN}------------------------------------------------------------------${NC}"
espflash flash --port "${PORT}" --monitor --release
