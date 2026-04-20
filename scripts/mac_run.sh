#!/usr/bin/env bash
set -euo pipefail

LOG_DIR="${LOG_DIR:-./logs}"
LOG_FILE="${LOG_FILE:-$LOG_DIR/mac_run.log}"
mkdir -p "$LOG_DIR"

echo "[mac_run] writing logs to $LOG_FILE"

RUST_LOG="${RUST_LOG:-gui=debug,app=debug,info}" \
  cargo run -p app --bin nodeimg --release -- "$@" 2>&1 | tee "$LOG_FILE"
