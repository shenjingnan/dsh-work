#!/usr/bin/env bash
# 按 plugins-version.txt 锁定的版本，用已预装的 dsh 在临时 DSH_HOME 里安装插件市场
# （dshmarket），把生成的 profiles/web 作为首启种子拷到 src-tauri/resources/profile-seed。
# 桌面应用首次启动时若检测到用户 DSH_HOME 尚无 profile，则整体种子化，开箱即有插件市场。
#
# 依赖：先跑 fetch-dsh.sh（需要 resources/dsh 的 dsh 入口），PATH 上需有 node 与 pnpm
# （dsh plugin 子命令硬编码 spawnSync("pnpm")）。
#
# 种子化的关键决策（spike 实测，见 docs/plans/2026-08-23-plugin-market-seed.md）：
# - profile 的 node_modules 是 hoisted 平铺布局，可整体跨机器复制；node_modules/.bin
#   下的符号链接用 cp -RL 解引用，产物断言零符号链接（Tauri resources 与 Rust 侧
#   fs::copy 均不保真符号链接）。
# - 只拷 profiles/web 子树，绝不碰 $DSH_HOME/profiles/node_modules（启动器维护的
#   绝对路径符号链接 fallback，dsh 每次启动自愈重建；被解引用复制会导致启动报错）。
# - 删除 node_modules/.modules.yaml：其中 storeDir 记录的是本机（CI）pnpm store 绝对
#   路径，跨机器后首次 pnpm add 触发 ERR_PNPM_UNEXPECTED_STORE 且无自动恢复；删除后
#   pnpm 将 node_modules 视为外来物，首次安装插件时全量重链（首装本就需联网，可接受）。
# - cordis.yml 是 dsh 启动时从 bundles + cordis.patch.yml 重算的派生文件，不入种子。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SPEC="$(tr -d '[:space:]' < "$ROOT/plugins-version.txt")"
NAME="${SPEC%@*}"
DEST="$ROOT/src-tauri/resources/profile-seed"
DSH_ENTRY="$ROOT/src-tauri/resources/dsh/node_modules/@deepseek-ai/dsh/lib/bin.js"

if [[ ! -f "$DSH_ENTRY" ]]; then
  echo "!! 未找到 dsh 入口 $DSH_ENTRY —— 请先运行 ./scripts/fetch-dsh.sh" >&2
  exit 1
fi
command -v node > /dev/null 2>&1 || { echo "!! PATH 上没有 node" >&2; exit 1; }
if ! command -v pnpm > /dev/null 2>&1; then
  echo "!! PATH 上没有 pnpm（dsh plugin 依赖它）。启用方式：corepack enable 或 npm i -g pnpm" >&2
  exit 1
fi

# CI 缓存命中时（actions/cache 按 plugins-version.txt + dsh-version.txt + 本脚本 hash
# 恢复）直接复用；用包内 manifest 的实际版本号复核，防止缓存内容与锁文件错位
if [[ -f "$DEST/web/node_modules/$NAME/package.json" ]]; then
  INSTALLED="$(node -p "require(process.argv[1]).version" "$DEST/web/node_modules/$NAME/package.json" 2>/dev/null || true)"
  if [[ "$INSTALLED" == "${SPEC#*@}" ]]; then
    echo ">> $SPEC 种子已存在（缓存命中），跳过"
    exit 0
  fi
fi

echo ">> 生成 $SPEC 种子"
SEED_HOME="$(mktemp -d "${TMPDIR:-/tmp}/dsh-plugin-seed.XXXXXX")"
trap 'rm -rf "$SEED_HOME"' EXIT

# 在干净的临时 DSH_HOME 里走官方链路安装：dsh plugin = pnpm 转发器 + bundle reconcile
DSH_HOME="$SEED_HOME" node "$DSH_ENTRY" plugin --profile web add "$SPEC"

# 防呆校验：依赖、bundle 层、lockfile 可移植性（全部用 node 断言，不依赖 jq —— Windows
# runner 的 Git Bash 没有 jq）
PROFILE_DIR="$SEED_HOME/profiles/web"
node -e '
const dir = process.argv[1], name = process.argv[2]
const manifest = require(require("path").join(dir, "package.json"))
const deps = Object.keys(manifest.dependencies ?? {})
if (!deps.includes(name)) { console.error(`!! 依赖列表缺少 ${name}: ${deps}`); process.exit(1) }
const bundles = manifest.dsh?.profile?.bundles ?? []
if (!bundles.includes(name)) { console.error(`!! bundles 层缺少 ${name}: ${bundles}`); process.exit(1) }
' "$PROFILE_DIR" "$NAME"
# 匹配须跟路径字符（./ ../ ~/ /），避免误伤 excludeLinksFromLockfile 这类设置项
if grep -nE '(file|link|portal):(\.|~|/)|git\+' "$PROFILE_DIR/pnpm-lock.yaml" > /dev/null; then
  echo "!! lockfile 含 file:/link:/git 本地引用，种子不可跨机器分发：" >&2
  grep -nE '(file|link|portal):(\.|~|/)|git\+' "$PROFILE_DIR/pnpm-lock.yaml" >&2
  exit 1
fi
if [[ ! -d "$PROFILE_DIR/node_modules/$NAME" ]]; then
  echo "!! 缺少 $PROFILE_DIR/node_modules/$NAME" >&2
  exit 1
fi

# 拷贝为种子：-L 解引用 node_modules/.bin 下的符号链接；仅 profiles/web 子树
rm -rf "$DEST"
mkdir -p "$DEST/web"
cp -RL "$PROFILE_DIR/." "$DEST/web/"

# dsh 启动时重算的派生文件与本机 pnpm store 痕迹不入种子（见头部注释）
rm -f "$DEST/web/cordis.yml"
rm -f "$DEST/web/node_modules/.modules.yaml"

# 防御性清理与 dsh 本体同源的平台原生变体（当前插件市场为纯 JS 无变体；保留此步
# 以防未来锁定的插件引入原生依赖，AppImage/linuxdeploy 约束见 fetch-dsh.sh 头注释）
find "$DEST/web/node_modules" -type d \
  \( -iname "*musl*" -o -iname "*linux-arm64*" \) \
  -prune -exec rm -rf {} +

# 种子必须零符号链接：Tauri resources 打包与 Rust 侧 fs::copy 都不保真链接
if [[ -n "$(find "$DEST" -type l)" ]]; then
  echo "!! 种子中存在符号链接（不应发生）：" >&2
  find "$DEST" -type l >&2
  exit 1
fi

echo ">> 完成：${DEST}（$(du -sh "$DEST" | cut -f1)）"
