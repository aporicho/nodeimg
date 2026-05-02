#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/windows_common.sh"

run_windows_existing "diag" "debug" "windows_diag_no_build_run" "$@"
