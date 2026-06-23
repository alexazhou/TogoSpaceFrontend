#!/usr/bin/env bash

set -euo pipefail

FRONTEND_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TOGOSPACE_ROOT="$(cd "$FRONTEND_DIR/.." && pwd)"

if [ ! -f "$TOGOSPACE_ROOT/src/backend_main.py" ]; then
  echo "未在上级目录找到 TogoSpace 仓库：$TOGOSPACE_ROOT"
  echo "请确认 frontend 位于 TogoSpace 的子目录结构下。"
  exit 1
fi

echo "当前桌面端直接使用上级 TogoSpace 仓库：$TOGOSPACE_ROOT"
echo "无需再初始化 external/TogoSpace 子仓库。"
