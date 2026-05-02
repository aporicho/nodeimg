#!/usr/bin/env bash
set -euo pipefail

# Compatibility wrapper.
# Default: user mode, release profile, cargo run.
# Pass --debug/--dev for developer mode.
# Pass --diag/--diagnostic for trace-level diagnostic mode.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "${1:-}" == "--diag" || "${1:-}" == "--diagnostic" ]]; then
  shift
  exec "$SCRIPT_DIR/windows_diag_build_run.sh" "$@"
fi

if [[ "${1:-}" == "--debug" || "${1:-}" == "--dev" ]]; then
  shift
  exec "$SCRIPT_DIR/windows_dev_build_run.sh" "$@"
fi

exec "$SCRIPT_DIR/windows_user_build_run.sh" "$@"
