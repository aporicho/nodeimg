#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "${1:-}" == "--diag" || "${1:-}" == "--diagnostic" ]]; then
  shift
  exec "$SCRIPT_DIR/mac_diag_build_run.sh" "$@"
fi

if [[ "${1:-}" == "--debug" || "${1:-}" == "--dev" ]]; then
  shift
  exec "$SCRIPT_DIR/mac_dev_build_run.sh" "$@"
fi

exec "$SCRIPT_DIR/mac_user_build_run.sh" "$@"
