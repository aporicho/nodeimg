#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_PACKAGE="app"
APP_BIN="nodeimg"

profile_flag() {
  local profile="$1"
  if [[ "$profile" == "release" ]]; then
    printf '%s' "--release"
  fi
}

default_rust_log() {
  local mode="$1"
  case "$mode" in
    dev)
      printf '%s' "gui=debug,app=debug,engine=debug,info"
      ;;
    diag)
      printf '%s' "gui=trace,app=trace,engine=trace,nodeimg=trace,info"
      ;;
    *)
      printf '%s' "info"
      ;;
  esac
}

default_rust_backtrace() {
  local mode="$1"
  if [[ "$mode" == "diag" ]]; then
    printf '%s' "1"
  else
    printf '%s' "0"
  fi
}

log_file_for() {
  local name="$1"
  local log_dir="${LOG_DIR:-$PROJECT_DIR/logs}"
  mkdir -p "$log_dir"
  printf '%s/%s.log' "$log_dir" "$name"
}

run_macos_cargo() {
  local mode="$1"
  local profile="$2"
  local name="$3"
  shift 3

  local rust_log="${RUST_LOG:-$(default_rust_log "$mode")}"
  local rust_backtrace="${RUST_BACKTRACE:-$(default_rust_backtrace "$mode")}"
  local log_file="${LOG_FILE:-$(log_file_for "$name")}"
  local cargo_profile_arg
  cargo_profile_arg="$(profile_flag "$profile")"

  echo "[$name] mode=$mode profile=$profile build=auto log=$log_file"

  cd "$PROJECT_DIR"
  if [[ -n "$cargo_profile_arg" ]]; then
    RUST_LOG="$rust_log" RUST_BACKTRACE="$rust_backtrace" cargo run -p "$APP_PACKAGE" --bin "$APP_BIN" "$cargo_profile_arg" -- "$@" 2>&1 | tee "$log_file"
  else
    RUST_LOG="$rust_log" RUST_BACKTRACE="$rust_backtrace" cargo run -p "$APP_PACKAGE" --bin "$APP_BIN" -- "$@" 2>&1 | tee "$log_file"
  fi
}

run_macos_existing() {
  local mode="$1"
  local profile="$2"
  local name="$3"
  shift 3

  local rust_log="${RUST_LOG:-$(default_rust_log "$mode")}"
  local rust_backtrace="${RUST_BACKTRACE:-$(default_rust_backtrace "$mode")}"
  local log_file="${LOG_FILE:-$(log_file_for "$name")}"
  local bin_path="$PROJECT_DIR/target/$profile/$APP_BIN"

  if [[ ! -x "$bin_path" ]]; then
    echo "[$name] missing executable: $bin_path" >&2
    echo "[$name] run scripts/mac_${mode}_build_run.sh first." >&2
    exit 1
  fi

  echo "[$name] mode=$mode profile=$profile build=none log=$log_file"

  cd "$PROJECT_DIR"
  RUST_LOG="$rust_log" RUST_BACKTRACE="$rust_backtrace" "$bin_path" "$@" 2>&1 | tee "$log_file"
}
