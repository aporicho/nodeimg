#!/usr/bin/env bash
# 通过 PowerShell 7 在 Windows 侧执行 cargo run --release
# 用法: ./scripts/run.sh [额外的 cargo 参数]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# 将 WSL 路径转换为 Windows 路径
WIN_PATH="$(wslpath -w "$PROJECT_DIR")"

/mnt/c/Users/pyura/scoop/shims/pwsh.exe -Command "Set-Location '$WIN_PATH'; cargo run --release $*"
