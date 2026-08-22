#!/usr/bin/env bash
# 从 src-tauri/icons/AppIcon.icon（Icon Composer 分层图标）编译 Liquid Glass 图标产物。
#
# 用法: scripts/build-icon.sh
#
# 产物（已提交到仓库，CI 无需重复执行）:
#   src-tauri/resources/Assets.car                        — 分层玻璃图标（macOS 26+ 动态渲染）
#   src-tauri/resources/AppIcon.icns                      — 旧系统扁平回退
#   src-tauri/resources/assetcatalog_generated_info.plist — actool 生成的图标键值参考
#
# 要求: Xcode 26+（.icon 格式与对应 actool 自 Xcode 26 起提供）。
# 修改图标请用 Icon Composer（Xcode 26 自带）编辑 icons/AppIcon.icon 后重跑本脚本。
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICON="$ROOT/src-tauri/icons/AppIcon.icon"
OUT="$ROOT/src-tauri/resources"

ACTOOL="$(xcrun -f actool 2>/dev/null || true)"
if [[ -z "$ACTOOL" ]]; then
  echo "!! 未找到 actool，请先安装 Xcode 26+ 并执行 sudo xcode-select -s /Applications/Xcode.app/Contents/Developer" >&2
  exit 1
fi

mkdir -p "$OUT"
"$ACTOOL" "$ICON" --compile "$OUT" \
  --output-format human-readable-text --notices --warnings --errors \
  --output-partial-info-plist "$OUT/assetcatalog_generated_info.plist" \
  --app-icon AppIcon --include-all-app-icons \
  --enable-on-demand-resources NO \
  --target-device mac --minimum-deployment-target 13.0 \
  --platform macosx

echo ">> 已生成 $OUT/Assets.car"
