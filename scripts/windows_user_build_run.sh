#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/windows_common.sh"

run_windows_cargo "user" "release" "windows_user_build_run" "$@"
