#!/usr/bin/env bash
# 按 dsh-version.txt 锁定的版本，把 @deepseek-ai/dsh 预装到 src-tauri/resources/dsh。
# 打包时该目录整体作为 Tauri resources 进入安装包。
#
# 优先用 pnpm（node-linker=hoisted 生成无符号链接的平铺布局，对打包友好，
# 且能命中本机 pnpm store，速度快）；没有 pnpm 时回退 npm。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(tr -d '[:space:]' < "$ROOT/dsh-version.txt")"
DEST="$ROOT/src-tauri/resources/dsh"

echo ">> 预装 @deepseek-ai/dsh@$VERSION 到 $DEST"
rm -rf "$DEST"
mkdir -p "$DEST"

if command -v pnpm > /dev/null 2>&1; then
  (cd "$DEST" && pnpm init > /dev/null 2>&1 || true)
  (cd "$DEST" && pnpm add "@deepseek-ai/dsh@$VERSION" \
    --node-linker=hoisted --config.confirmModulesPurge=false)
else
  npm install --prefix "$DEST" "@deepseek-ai/dsh@$VERSION" --omit=dev --no-audit --no-fund
fi

ENTRY="$DEST/node_modules/@deepseek-ai/dsh/lib/bin.js"
if [[ ! -f "$ENTRY" ]]; then
  echo "!! 预装失败：缺少入口 $ENTRY" >&2
  exit 1
fi
echo ">> 完成：$ENTRY"
