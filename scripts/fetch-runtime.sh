#!/usr/bin/env bash
# 下载内置运行时：Node.js 官方单二进制（按目标 triple）+ pnpm 单文件分发。
#
# 用法: scripts/fetch-runtime.sh <target-triple>
#   aarch64-apple-darwin | x86_64-apple-darwin | x86_64-pc-windows-msvc | x86_64-unknown-linux-gnu
#
# 产物:
#   src-tauri/binaries/node-<triple>[.exe]  — Tauri externalBin（安装为主程序旁的 node）
#   src-tauri/resources/pnpm/               — pnpm.cjs + pnpm / pnpm.cmd shim
set -euo pipefail

NODE_VERSION="v22.22.2"
PNPM_VERSION="10.34.5"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRIPLE="${1:?用法: $0 <target-triple>}"

case "$TRIPLE" in
  aarch64-apple-darwin)      NODE_DIST="darwin-arm64"; NODE_PKG="tar.gz" ;;
  x86_64-apple-darwin)       NODE_DIST="darwin-x64";   NODE_PKG="tar.gz" ;;
  x86_64-unknown-linux-gnu)  NODE_DIST="linux-x64";    NODE_PKG="tar.gz" ;;
  x86_64-pc-windows-msvc)    NODE_DIST="win-x64";      NODE_PKG="exe" ;;
  *) echo "!! 未知 target triple: $TRIPLE" >&2; exit 1 ;;
esac

BINARIES="$ROOT/src-tauri/binaries"
RESOURCES_PNPM="$ROOT/src-tauri/resources/pnpm"
mkdir -p "$BINARIES" "$RESOURCES_PNPM"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# --- Node ---
if [[ "$NODE_PKG" == "exe" ]]; then
  # Windows：官方直接提供 node.exe 单文件
  curl -fSL "https://nodejs.org/dist/$NODE_VERSION/$NODE_DIST/node.exe" \
    -o "$BINARIES/node-$TRIPLE.exe"
else
  curl -fSL "https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-$NODE_DIST.tar.gz" \
    -o "$TMP/node.tar.gz"
  tar -xzf "$TMP/node.tar.gz" -C "$TMP"
  cp "$TMP/node-$NODE_VERSION-$NODE_DIST/bin/node" "$BINARIES/node-$TRIPLE"
  chmod +x "$BINARIES/node-$TRIPLE"
fi
echo ">> node $NODE_VERSION ($NODE_DIST) -> $BINARIES/node-$TRIPLE$([[ "$NODE_PKG" == "exe" ]] && echo .exe)"

# --- pnpm（单文件分发，经内置 node 执行；仅首次下载一次，各 triple 通用） ---
if [[ ! -f "$RESOURCES_PNPM/pnpm.cjs" ]]; then
  curl -fSL "https://registry.npmjs.org/pnpm/-/pnpm-$PNPM_VERSION.tgz" -o "$TMP/pnpm.tgz"
  tar -xzf "$TMP/pnpm.tgz" -C "$TMP" package/dist/pnpm.cjs
  cp "$TMP/package/dist/pnpm.cjs" "$RESOURCES_PNPM/pnpm.cjs"

  # unix shim：dsh 以 spawnSync("pnpm") 调用，需要 PATH 上有可执行的 pnpm
  cat > "$RESOURCES_PNPM/pnpm" <<'SHIM'
#!/bin/sh
exec node "$(dirname "$0")/pnpm.cjs" "$@"
SHIM
  chmod +x "$RESOURCES_PNPM/pnpm"

  # Windows shim：dsh 在 win32 上以 shell:true 调用，.cmd 可被 cmd 解析
  printf '@echo off\r\nnode "%%~dp0pnpm.cjs" %%*\r\n' > "$RESOURCES_PNPM/pnpm.cmd"
  echo ">> pnpm $PNPM_VERSION -> $RESOURCES_PNPM"
else
  echo ">> pnpm $PNPM_VERSION 已存在，跳过"
fi
