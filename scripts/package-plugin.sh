#!/bin/bash
#
# Terminal Plugin 打包脚本
#
# 将插件打包为 zip 格式，用于在 ProxyCast 插件中心安装
#
# 用法:
#   ./scripts/package-plugin.sh          # 打包插件
#   ./scripts/package-plugin.sh 0.2.0    # 指定版本号
#

set -e

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# 项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PLUGIN_DIR="$PROJECT_ROOT/plugin"
OUTPUT_DIR="$PROJECT_ROOT/dist"

# 版本号
VERSION_OVERRIDE="$1"

# 读取版本号
if [ -n "$VERSION_OVERRIDE" ]; then
    VERSION="$VERSION_OVERRIDE"
    log_info "使用指定版本: $VERSION"
else
    VERSION=$(grep -o '"version"[[:space:]]*:[[:space:]]*"[^"]*"' "$PLUGIN_DIR/plugin.json" | head -1 | sed 's/.*"\([^"]*\)"$/\1/')
    if [ -z "$VERSION" ]; then
        VERSION="0.1.0"
    fi
    log_info "从 plugin.json 读取版本: $VERSION"
fi

# 创建输出目录
mkdir -p "$OUTPUT_DIR"

# 输出文件名
OUTPUT_FILE="$OUTPUT_DIR/terminal-plugin.zip"

log_info "开始打包插件..."

# 先构建前端
log_info "构建前端..."
cd "$PROJECT_ROOT"
npm run build

# 检查前端构建产物
if [ ! -f "$PLUGIN_DIR/dist/index.js" ]; then
    log_warn "前端构建产物不存在，请先运行 npm run build"
    exit 1
fi

# 创建临时目录
TEMP_DIR=$(mktemp -d)
TEMP_PLUGIN_DIR="$TEMP_DIR/terminal-plugin"

log_info "复制插件文件..."
mkdir -p "$TEMP_PLUGIN_DIR/dist"

# 复制配置文件
cp "$PLUGIN_DIR/plugin.json" "$TEMP_PLUGIN_DIR/"
cp "$PLUGIN_DIR/config.json" "$TEMP_PLUGIN_DIR/"

# 复制前端构建产物
cp "$PLUGIN_DIR/dist/index.js" "$TEMP_PLUGIN_DIR/dist/"
cp "$PLUGIN_DIR/dist/styles.css" "$TEMP_PLUGIN_DIR/dist/" 2>/dev/null || true

# 如果指定了版本覆盖，更新 plugin.json
if [ -n "$VERSION_OVERRIDE" ]; then
    log_info "更新 plugin.json 版本号..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/\"version\"[[:space:]]*:[[:space:]]*\"[^\"]*\"/\"version\": \"$VERSION_OVERRIDE\"/" "$TEMP_PLUGIN_DIR/plugin.json"
    else
        sed -i "s/\"version\"[[:space:]]*:[[:space:]]*\"[^\"]*\"/\"version\": \"$VERSION_OVERRIDE\"/" "$TEMP_PLUGIN_DIR/plugin.json"
    fi
fi

# 删除旧的输出文件
rm -f "$OUTPUT_FILE"

# 创建 zip 包
log_info "创建 zip 包..."
cd "$TEMP_DIR"
zip -r "$OUTPUT_FILE" "terminal-plugin" -x "*.DS_Store" -x "*__MACOSX*"

# 清理临时目录
rm -rf "$TEMP_DIR"

# 计算校验和
if command -v sha256sum &> /dev/null; then
    CHECKSUM=$(sha256sum "$OUTPUT_FILE" | awk '{print $1}')
elif command -v shasum &> /dev/null; then
    CHECKSUM=$(shasum -a 256 "$OUTPUT_FILE" | awk '{print $1}')
else
    CHECKSUM="N/A"
fi

# 输出结果
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}插件打包完成！${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "插件名称: terminal-plugin"
echo "版本: $VERSION"
echo "输出文件: $OUTPUT_FILE"
echo "文件大小: $(du -h "$OUTPUT_FILE" | cut -f1)"
echo "SHA256: $CHECKSUM"
echo ""
echo "安装方式:"
echo "  1. 在 ProxyCast 中打开「插件中心」"
echo "  2. 点击「安装插件」"
echo "  3. 选择本地文件: $OUTPUT_FILE"
echo ""
echo "或者使用 URL 安装（需要先上传到服务器）"
