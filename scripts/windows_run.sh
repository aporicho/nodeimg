#!/usr/bin/env bash
set -euo pipefail

# 通过 PowerShell 7 在 Windows 侧执行 cargo run --release
# 用法: ./scripts/windows_run.sh [额外的 cargo 参数]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_DIR="${LOG_DIR:-$PROJECT_DIR/logs}"
LOG_FILE="${LOG_FILE:-$LOG_DIR/windows_run.log}"

mkdir -p "$LOG_DIR"

echo "[windows_run] writing logs to $LOG_FILE"

# 将 WSL 路径转换为 Windows 路径
WIN_PATH="$(wslpath -w "$PROJECT_DIR")"

# 可执行入口位于 app 包的 nodeimg bin
/mnt/c/Users/pyura/scoop/shims/pwsh.exe -Command "Set-Location '$WIN_PATH'; \$env:RUST_LOG='gui=debug,app=debug,info'; cargo run -p app --bin nodeimg --release -- $*" 2>&1 | tee "$LOG_FILE"
