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

# --- Node（CI 缓存命中时跳过下载） ---
# 注意：不能用 VAR=...$( [[ ]] && echo ) 的内联写法，命令替换失败会让
# set -e 直接退出脚本
NODE_BIN="$BINARIES/node-$TRIPLE"
if [[ "$NODE_PKG" == "exe" ]]; then
  NODE_BIN="$NODE_BIN.exe"
fi
if [[ -f "$NODE_BIN" ]]; then
  echo ">> node $NODE_VERSION ($NODE_DIST) 已存在（缓存命中），跳过"
elif [[ "$NODE_PKG" == "exe" ]]; then
  # Windows：官方直接提供 node.exe 单文件
  curl -fSL "https://nodejs.org/dist/$NODE_VERSION/$NODE_DIST/node.exe" -o "$NODE_BIN"
  echo ">> node $NODE_VERSION ($NODE_DIST) -> $NODE_BIN"
else
  curl -fSL "https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-$NODE_DIST.tar.gz" \
    -o "$TMP/node.tar.gz"
  tar -xzf "$TMP/node.tar.gz" -C "$TMP"
  cp "$TMP/node-$NODE_VERSION-$NODE_DIST/bin/node" "$NODE_BIN"
  chmod +x "$NODE_BIN"
  echo ">> node $NODE_VERSION ($NODE_DIST) -> $NODE_BIN"
fi

# --- pnpm（单文件分发，经内置 node 执行；仅首次下载一次，各 triple 通用） ---
# dist/worker.js 必须随 pnpm.cjs 一起分发：pnpm 10.3x 把 resolve/fetch 放在
# worker 线程执行，worker 脚本按 __dirname 相对解析（pnpm.cjs 内
# workerScriptPath = join(__dirname, "worker.js")）。缺失时 add 表现为静默
# 无操作（added 0、不写 package.json、exit 0）或 "Worker pnpm#1 exited with
# code 1"，曾导致内置 pnpm 完全无法安装插件。dist/pnpmrc 是内置默认配置，一并随行。
if [[ ! -f "$RESOURCES_PNPM/pnpm.cjs" || ! -f "$RESOURCES_PNPM/worker.js" ]]; then
  curl -fSL "https://registry.npmjs.org/pnpm/-/pnpm-$PNPM_VERSION.tgz" -o "$TMP/pnpm.tgz"
  tar -xzf "$TMP/pnpm.tgz" -C "$TMP" \
    package/dist/pnpm.cjs package/dist/worker.js package/dist/pnpmrc
  cp "$TMP/package/dist/pnpm.cjs" "$TMP/package/dist/worker.js" "$TMP/package/dist/pnpmrc" \
    "$RESOURCES_PNPM/"

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
