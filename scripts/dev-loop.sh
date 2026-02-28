#!/usr/bin/env bash
#
# Development loop helper:
# - sim:   run PC simulator
# - flash: build + flash + monitor for ESP32-C3
# - check: run host tests + ESP32 build check

set -euo pipefail

SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
TARGET_TRIPLE="${TARGET_TRIPLE:-riscv32imc-unknown-none-elf}"
BOARD_FEATURE="${BOARD_FEATURE:-esp32c3}"
PACKAGE_NAME="platform-esp32"
BIN_NAME="platform-esp32"
MIN_RUST_VERSION="1.82.0"

usage() {
  cat <<EOF
Usage: ${SCRIPT_NAME} <sim|flash|check> [serial_port]

Commands:
  sim                 Run PC simulator (platform-pc-sim)
  flash [serial_port] Build + flash + monitor on ESP32-C3
  check               Run host tests and ESP32 cross-build check

Environment overrides:
  TARGET_TRIPLE       Default: ${TARGET_TRIPLE}
  BOARD_FEATURE       Default: ${BOARD_FEATURE}
EOF
}

ensure_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Error: command not found: $1" >&2
    exit 1
  fi
}

ensure_rust_target() {
  ensure_command rustup
  if ! rustup target list --installed | grep -qx "${TARGET_TRIPLE}"; then
    echo "Error: rust target '${TARGET_TRIPLE}' is not installed." >&2
    echo "Install with: rustup target add ${TARGET_TRIPLE}" >&2
    exit 1
  fi
}

version_ge() {
  local found="${1}"
  local required="${2}"
  local f_major f_minor f_patch r_major r_minor r_patch

  IFS='.' read -r f_major f_minor f_patch <<<"${found}"
  IFS='.' read -r r_major r_minor r_patch <<<"${required}"
  f_patch="${f_patch:-0}"
  r_patch="${r_patch:-0}"

  if (( f_major > r_major )); then
    return 0
  fi
  if (( f_major < r_major )); then
    return 1
  fi
  if (( f_minor > r_minor )); then
    return 0
  fi
  if (( f_minor < r_minor )); then
    return 1
  fi
  (( f_patch >= r_patch ))
}

ensure_rust_version() {
  ensure_command rustc
  local current
  current="$(rustc --version | awk '{print $2}')"

  if ! version_ge "${current}" "${MIN_RUST_VERSION}"; then
    echo "Error: rustc ${MIN_RUST_VERSION}+ is required for ESP32 build path." >&2
    echo "Current rustc: ${current}" >&2
    echo "Update with: rustup update stable" >&2
    exit 1
  fi
}

run_sim() {
  cargo run -p platform-pc-sim
}

run_flash() {
  local serial_port="${1:-}"
  local artifact="target/${TARGET_TRIPLE}/release/${BIN_NAME}"
  local flash_args=(flash --monitor --chip "${BOARD_FEATURE}")

  ensure_command cargo
  ensure_command espflash
  ensure_rust_version
  ensure_rust_target

  cargo build \
    -p "${PACKAGE_NAME}" \
    --release \
    --target "${TARGET_TRIPLE}" \
    --no-default-features \
    --features "${BOARD_FEATURE}"

  if [[ ! -f "${artifact}" ]]; then
    echo "Error: built artifact not found: ${artifact}" >&2
    exit 1
  fi

  if [[ -n "${serial_port}" ]]; then
    flash_args+=(--port "${serial_port}")
  fi
  flash_args+=("${artifact}")

  espflash "${flash_args[@]}"
}

run_check() {
  ensure_command cargo
  ensure_rust_version
  ensure_rust_target

  cargo test --all
  cargo build \
    -p "${PACKAGE_NAME}" \
    --release \
    --target "${TARGET_TRIPLE}" \
    --no-default-features \
    --features "${BOARD_FEATURE}"
}

main() {
  local command="${1:-}"
  case "${command}" in
    sim)
      run_sim
      ;;
    flash)
      run_flash "${2:-}"
      ;;
    check)
      run_check
      ;;
    -h|--help|"")
      usage
      ;;
    *)
      echo "Error: unknown command '${command}'" >&2
      usage
      exit 1
      ;;
  esac
}

main "$@"
