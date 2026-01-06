#!/bin/bash
#
# Terminal Plugin 本地构建脚本
#
# 用于本地开发和测试，构建当前平台的二进制文件
#
# 用法:
#   ./scripts/build-local.sh          # 构建 debug 版本
#   ./scripts/build-local.sh --release # 构建 release 版本
#

set -e

# 颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# 项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# 解析参数
RELEASE_FLAG=""
if [ "$1" = "--release" ]; then
    RELEASE_FLAG="--release"
fi

# 构建前端
log_info "构建前端..."
cd "$PROJECT_ROOT"
npm run build

# 构建后端
log_info "构建后端..."
cd "$PROJECT_ROOT/src-tauri"
cargo build $RELEASE_FLAG

log_success "构建完成!"
echo ""
echo "前端输出: plugin/dist/"
if [ -n "$RELEASE_FLAG" ]; then
    echo "后端输出: src-tauri/target/release/terminal-plugin-cli"
else
    echo "后端输出: src-tauri/target/debug/terminal-plugin-cli"
fi
