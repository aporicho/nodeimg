#!/usr/bin/env bash
set -euo pipefail

# 通过 PowerShell 7 在 Windows 侧执行 cargo run。
# 默认使用 release；传 --debug/--dev 可启用 debug 构建和开发者模式。
# 用法: ./scripts/windows_run.sh [--debug|--dev] [额外的 cargo 参数]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="${LOG_DIR:-$PROJECT_DIR/logs}"
LOG_FILE="${LOG_FILE:-$LOG_DIR/windows_run.log}"
PROFILE="release"

if [[ "${1:-}" == "--debug" || "${1:-}" == "--dev" ]]; then
  PROFILE="debug"
  shift
fi

if [[ "$PROFILE" == "release" ]]; then
  CARGO_PROFILE_ARG="--release"
else
  CARGO_PROFILE_ARG=""
fi

mkdir -p "$LOG_DIR"

echo "[windows_run] profile=$PROFILE writing logs to $LOG_FILE"

# 将 WSL 路径转换为 Windows 路径
WIN_PATH="$(wslpath -w "$PROJECT_DIR")"
WIN_TARGET_PATTERN="${WIN_PATH}\\target\\*"
PWSH="/mnt/c/Users/pyura/scoop/shims/pwsh.exe"

cleanup_workspace_nodeimg() {
  "$PWSH" -Command "Get-Process nodeimg -ErrorAction SilentlyContinue | Where-Object { \$_.Path -like '$WIN_TARGET_PATTERN' } | Stop-Process -ErrorAction SilentlyContinue" >/dev/null 2>&1 || true
}

trap cleanup_workspace_nodeimg EXIT INT TERM
cleanup_workspace_nodeimg

# 可执行入口位于 app 包的 nodeimg bin
"$PWSH" -Command "Set-Location '$WIN_PATH'; \$env:RUST_LOG='gui=debug,app=debug,info'; \$env:CARGO_INCREMENTAL='0'; cargo run -p app --bin nodeimg $CARGO_PROFILE_ARG -- $*" 2>&1 | tee "$LOG_FILE"
