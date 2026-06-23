#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
用法：
  ./scripts/dev-desktop.sh [--skip-install] [--help] [-- <tauri dev 参数>]

说明：
  - 默认使用上级 TogoSpace 仓库作为 TOGOSPACE_ROOT
  - 自动补齐 apps/desktop/.env
  - 默认执行 npm install，然后调用 tauri dev

示例：
  ./scripts/dev-desktop.sh
  ./scripts/dev-desktop.sh --skip-install
  ./scripts/dev-desktop.sh -- --no-watch
EOF
}

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DESKTOP_DIR="$ROOT_DIR/apps/desktop"
DEFAULT_TOGOSPACE_ROOT="$(cd "$ROOT_DIR/.." && pwd)"
TOGOSPACE_ROOT="${TOGOSPACE_ROOT:-$DEFAULT_TOGOSPACE_ROOT}"
SKIP_INSTALL=0
TAURI_ARGS=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    --skip-install)
      SKIP_INSTALL=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --)
      shift
      TAURI_ARGS=("$@")
      break
      ;;
    *)
      TAURI_ARGS+=("$1")
      shift
      ;;
  esac
done

if [ ! -f "$TOGOSPACE_ROOT/src/backend_main.py" ]; then
  echo "未找到可运行的 TogoSpace 仓库：$TOGOSPACE_ROOT" >&2
  echo "请确认当前 frontend 仓库位于 TogoSpace 的子目录下，或手动设置 TOGOSPACE_ROOT。" >&2
  exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "当前环境缺少 npm，请先安装 Node.js。" >&2
  exit 1
fi

export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo >/dev/null 2>&1; then
  echo "当前环境缺少 cargo，请先安装 Rust。" >&2
  echo "可参考 apps/desktop/README.md 中的 Rust 安装命令。" >&2
  exit 1
fi

cd "$DESKTOP_DIR"

if [ ! -f ".env" ] && [ -f ".env.example" ]; then
  cp .env.example .env
fi

if [ "$SKIP_INSTALL" -eq 0 ]; then
  npm install
fi

echo "使用 TogoSpace 路径：$TOGOSPACE_ROOT"
export TOGOSPACE_ROOT

if [ "${#TAURI_ARGS[@]}" -gt 0 ]; then
  npm run tauri dev -- "${TAURI_ARGS[@]}"
else
  npm run tauri dev
fi
