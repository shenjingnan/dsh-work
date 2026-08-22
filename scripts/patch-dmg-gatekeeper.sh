#!/usr/bin/env bash
# 向 dmg 安装镜像注入「首次打开修复.command」。
#
# 背景：DSHWork 未经 Apple 签名公证，新版 macOS 双击打开会被 Gatekeeper 拦截，
# 报「"DSHWork" 已损坏，无法打开」（实为隔离属性作祟，并非真损坏），需执行
#   xattr -cr /Applications/DSHWork.app
# 把修复命令做成 dmg 里的双击脚本，用户拖完 Applications 后双击即可完成，
# 不必先去阅读 README。
#
# 用法: patch-dmg-gatekeeper.sh <xxx.dmg>   （仅 macOS，依赖 hdiutil）
#
# 实现：Tauri 没有往 dmg 根目录放额外文件的配置（bundle.macOS.files 进的是
# .app 内部），只能在产物 dmg 上后处理：挂载 → 连同 .DS_Store / .background
# 原样拷出（保住安装窗口布局）→ 加入修复脚本 → hdiutil 重建压缩镜像。
set -euo pipefail

DMG="${1:?用法: $0 <dmg 路径>}"
VOL_NAME="DSHWork" # Tauri 的 dmg 卷名 = productName，重建时保持一致
FIXER_NAME="首次打开修复.command"

WORK="$(mktemp -d)"
STAGE="$WORK/stage"
MNT="$WORK/mnt"
mkdir -p "$STAGE"

# 挂载原镜像并全量拷贝（含 .DS_Store/.background 隐藏文件与 Applications 符号链接，
# 缺了它们安装窗口的布局/背景就没了）
hdiutil attach "$DMG" -mountpoint "$MNT" -nobrowse -readonly -quiet
cp -a "$MNT"/. "$STAGE"/
hdiutil detach "$MNT" -quiet

cat > "$STAGE/$FIXER_NAME" <<'FIXER'
#!/bin/bash
# 双击本文件修复「DSHWork 已损坏，无法打开」提示（清除 Gatekeeper 隔离属性）。
# 前提：已把 DSHWork 拖入 Applications 文件夹。
clear
APP="/Applications/DSHWork.app"
echo "==> 正在修复 DSHWork 首次打开问题 ..."
if [ ! -d "$APP" ]; then
  echo "!! 未找到 $APP"
  echo "    请先把 DSHWork 拖入 Applications 文件夹，再重新双击本文件。"
  echo "    （按任意键关闭）"
  read -n 1 -s
  exit 1
fi
if xattr -cr "$APP"; then
  echo "==> 修复完成，正在启动 DSHWork ..."
  sleep 1
  open "$APP"
  sleep 5
else
  echo "!! 修复失败：请手动执行  xattr -cr $APP  或到 GitHub Issues 反馈"
  echo "    （按任意键关闭）"
  read -n 1 -s
fi
FIXER
chmod +x "$STAGE/$FIXER_NAME"

# 重建压缩镜像，覆盖原文件。用 ULMO（LZMA）而非 Tauri 默认的 UDZO（zlib）：
# 同 payload 实测 94.9MB → 68.5MB（-28%）。ULMO 挂载需 macOS 10.15+，
# 本项目 minimumSystemVersion 13.0，无兼容性问题。
rm "$DMG"
hdiutil create -volname "$VOL_NAME" -srcfolder "$STAGE" -ov -format ULMO -quiet "$DMG"
echo "已注入 $FIXER_NAME -> $DMG"
