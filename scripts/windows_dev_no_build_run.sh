#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/windows_common.sh"

run_windows_existing "dev" "debug" "windows_dev_no_build_run" "$@"
