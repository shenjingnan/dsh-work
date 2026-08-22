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
ENTRY="$DEST/node_modules/@deepseek-ai/dsh/lib/bin.js"

# CI 缓存命中时（actions/cache 按 dsh-version.txt 的 hash 恢复，版本已对齐）
# 直接复用，跳过下载与 musl/arm64 清理（缓存保存的是清理后的最终状态）
if [[ -f "$ENTRY" ]]; then
  echo ">> dsh@$VERSION 已预装（缓存命中），跳过"
  exit 0
fi

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

# 清除与发布目标无关的平台原生二进制变体。AppImage 打包时 linuxdeploy 会
# 递归扫描 AppDir 内所有 ELF 并部署动态依赖：musl 变体依赖
# libc.musl-x86_64.so.1，glibc 环境下必然找不到而打包失败；linux-arm64
# 变体在 x64 runner 上触发 patchelf 报错。本项目发布矩阵不含这两类目标
# （macOS/Windows 不经过 linuxdeploy，删除同样无影响）。
find "$DEST/node_modules" -type d \
  \( -iname "*musl*" -o -iname "*linux-arm64*" \) \
  -prune -exec rm -rf {} +
echo ">> 已清除 musl / linux-arm64 平台变体"

if [[ ! -f "$ENTRY" ]]; then
  echo "!! 预装失败：缺少入口 $ENTRY" >&2
  exit 1
fi
echo ">> 完成：$ENTRY"
