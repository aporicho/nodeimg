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
  if [[ "$mode" == "dev" ]]; then
    printf '%s' "gui=debug,app=debug,engine=debug,info"
  else
    printf '%s' "info"
  fi
}

log_file_for() {
  local name="$1"
  local log_dir="${LOG_DIR:-$PROJECT_DIR/logs}"
  mkdir -p "$log_dir"
  printf '%s/%s.log' "$log_dir" "$name"
}

find_pwsh() {
  if [[ -n "${PWSH:-}" ]]; then
    printf '%s' "$PWSH"
    return
  fi

  local candidates=(
    "/mnt/c/Users/pyura/scoop/shims/pwsh.exe"
    "/mnt/c/Program Files/PowerShell/7/pwsh.exe"
    "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe"
  )

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -x "$candidate" ]]; then
      printf '%s' "$candidate"
      return
    fi
  done

  if command -v pwsh.exe >/dev/null 2>&1; then
    command -v pwsh.exe
    return
  fi

  if command -v powershell.exe >/dev/null 2>&1; then
    command -v powershell.exe
    return
  fi

  echo "PowerShell executable not found. Set PWSH=/path/to/pwsh.exe." >&2
  exit 1
}

ps_quote() {
  local value="${1//\'/\'\'}"
  printf "'%s'" "$value"
}

ps_args() {
  local first=1
  local arg
  for arg in "$@"; do
    if [[ "$first" -eq 0 ]]; then
      printf ' '
    fi
    first=0
    ps_quote "$arg"
  done
}

ps_prefixed_env_assignments() {
  local prefix="$1"
  local name
  local value
  env | while IFS='=' read -r name value; do
    if [[ "$name" == "$prefix"* && "$name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
      printf '$env:%s=%s; ' "$name" "$(ps_quote "$value")"
    fi
  done
}

windows_project_path() {
  if ! command -v wslpath >/dev/null 2>&1; then
    echo "wslpath is required by the Windows launcher scripts." >&2
    exit 1
  fi
  wslpath -w "$PROJECT_DIR"
}

cleanup_workspace_nodeimg() {
  local pwsh="$1"
  local win_project="$2"
  local win_target_pattern="${win_project}\\target\\*"
  "$pwsh" -Command "Get-Process nodeimg -ErrorAction SilentlyContinue | Where-Object { \$_.Path -like $(ps_quote "$win_target_pattern") } | Stop-Process -ErrorAction SilentlyContinue" >/dev/null 2>&1 || true
}

run_windows_cargo() {
  local mode="$1"
  local profile="$2"
  local name="$3"
  shift 3

  local rust_log="${RUST_LOG:-$(default_rust_log "$mode")}"
  local log_file="${LOG_FILE:-$(log_file_for "$name")}"
  local cargo_profile_arg
  local pwsh
  local win_project
  local app_args
  local app_env

  cargo_profile_arg="$(profile_flag "$profile")"
  pwsh="$(find_pwsh)"
  win_project="$(windows_project_path)"
  app_args="$(ps_args "$@")"
  app_env="$(ps_prefixed_env_assignments "NODEIMG_")"

  echo "[$name] mode=$mode profile=$profile build=auto log=$log_file"

  cleanup_workspace_nodeimg "$pwsh" "$win_project"
  trap 'cleanup_workspace_nodeimg "'"$pwsh"'" "'"$win_project"'"' EXIT INT TERM

  "$pwsh" -Command "Set-Location $(ps_quote "$win_project"); ${app_env}\$env:RUST_LOG=$(ps_quote "$rust_log"); \$env:CARGO_INCREMENTAL='0'; cargo run -p $APP_PACKAGE --bin $APP_BIN $cargo_profile_arg -- $app_args" 2>&1 | tee "$log_file"
}

run_windows_existing() {
  local mode="$1"
  local profile="$2"
  local name="$3"
  shift 3

  local rust_log="${RUST_LOG:-$(default_rust_log "$mode")}"
  local log_file="${LOG_FILE:-$(log_file_for "$name")}"
  local pwsh
  local win_project
  local win_bin
  local app_args
  local app_env

  pwsh="$(find_pwsh)"
  win_project="$(windows_project_path)"
  win_bin="${win_project}\\target\\${profile}\\${APP_BIN}.exe"
  app_args="$(ps_args "$@")"
  app_env="$(ps_prefixed_env_assignments "NODEIMG_")"

  echo "[$name] mode=$mode profile=$profile build=none log=$log_file"

  cleanup_workspace_nodeimg "$pwsh" "$win_project"
  trap 'cleanup_workspace_nodeimg "'"$pwsh"'" "'"$win_project"'"' EXIT INT TERM

  "$pwsh" -Command "Set-Location $(ps_quote "$win_project"); if (!(Test-Path $(ps_quote "$win_bin"))) { Write-Error 'Missing executable: $win_bin. Run scripts/windows_${mode}_build_run.sh first.'; exit 1 }; ${app_env}\$env:RUST_LOG=$(ps_quote "$rust_log"); & $(ps_quote "$win_bin") $app_args" 2>&1 | tee "$log_file"
}
