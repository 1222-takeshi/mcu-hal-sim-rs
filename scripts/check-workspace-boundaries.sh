#!/usr/bin/env bash
# check-workspace-boundaries.sh
#
# Enforces the layered dependency rules for mcu-hal-sim-rs:
#
#   hal-api        — no dependencies on platform-* or core-app
#   core-app       — no dependencies on platform-* or reference-drivers
#   reference-drivers — no dependencies on platform-*
#
# Each check scans both Cargo.toml and Rust source files so that
# accidental `extern crate` / `use` imports are also caught.
#
# Exit code: 0 = all boundaries respected, 1 = violation found.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

red()  { echo -e "\033[31m$*\033[0m"; }
green(){ echo -e "\033[32m$*\033[0m"; }

check_no_dep() {
    local crate_dir="$1"      # e.g. crates/hal-api
    local crate_name="$2"     # e.g. hal-api
    local forbidden="$3"      # grep pattern, e.g. 'platform-'
    local friendly="$4"       # human-readable forbidden label

    local toml_hits
    toml_hits=$(grep -r "$forbidden" "$REPO_ROOT/$crate_dir/Cargo.toml" 2>/dev/null || true)
    if [[ -n "$toml_hits" ]]; then
        red "FAIL [$crate_name] Cargo.toml contains a '$friendly' dependency:"
        echo "$toml_hits"
        FAIL=1
    fi

    # Only check actual import/use statements in source code (not doc comments).
    # Crate names use underscores in `use`/`extern crate`; convert hyphens for matching.
    local src_pattern
    src_pattern=$(echo "$forbidden" | sed 's/-/_/g')
    local src_hits
    src_hits=$(grep -rn --include="*.rs" -E \
                   "^[[:space:]]*(use|extern[[:space:]]+crate)[[:space:]].*${src_pattern}" \
                   "$REPO_ROOT/$crate_dir" 2>/dev/null || true)
    if [[ -n "$src_hits" ]]; then
        red "FAIL [$crate_name] source code imports '$friendly':"
        echo "$src_hits"
        FAIL=1
    fi
}

echo "=== Workspace boundary check ==="

# hal-api must not reference platform-* or core-app
check_no_dep "crates/hal-api" "hal-api" "platform-" "platform-*"
check_no_dep "crates/hal-api" "hal-api" "core-app\|core_app" "core-app"

# core-app must not reference platform-* or reference-drivers
check_no_dep "crates/core-app" "core-app" "platform-" "platform-*"
check_no_dep "crates/core-app" "core-app" "reference-drivers\|reference_drivers" "reference-drivers"

# reference-drivers must not reference platform-*
check_no_dep "crates/reference-drivers" "reference-drivers" "platform-" "platform-*"

if [[ "$FAIL" -eq 0 ]]; then
    green "All workspace boundaries respected."
else
    red "One or more boundary violations detected. See above."
    exit 1
fi
