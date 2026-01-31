#!/bin/bash
# anyFAST macOS Helper 安装脚本
# 此脚本用于安装 setuid helper 二进制文件

set -e

HELPER_NAME="anyfast-helper-macos"
INSTALL_PATH="/usr/local/bin/$HELPER_NAME"

echo "=========================================="
echo "  anyFAST macOS Helper 安装程序"
echo "=========================================="
echo ""

# 检查是否在 macOS 上运行
if [[ "$(uname)" != "Darwin" ]]; then
    echo "错误: 此脚本仅适用于 macOS"
    exit 1
fi

# 检查 helper 是否已存在于常见位置
BINARY_PATH=""
POSSIBLE_PATHS=(
    "./anyfast-helper-macos"
    "./target/release/anyfast-helper-macos"
    "../target/release/anyfast-helper-macos"
    "$(dirname "$0")/../src-tauri/target/release/anyfast-helper-macos"
)

for path in "${POSSIBLE_PATHS[@]}"; do
    if [[ -f "$path" ]]; then
        BINARY_PATH="$path"
        break
    fi
done

if [[ -z "$BINARY_PATH" ]]; then
    echo "未找到预编译的 helper 二进制文件"
    echo ""
    echo "请先编译 helper:"
    echo "  cd rust/src-tauri"
    echo "  cargo build --release --bin anyfast-helper-macos"
    echo ""
    echo "然后重新运行此脚本"
    exit 1
fi

echo "找到 helper: $BINARY_PATH"
echo ""

echo "步骤 1: 安装 helper 到 $INSTALL_PATH..."
echo "需要管理员权限来设置 setuid 位"
echo ""

# 复制到 /usr/local/bin
sudo cp "$BINARY_PATH" "$INSTALL_PATH"

# 设置所有权为 root:wheel
sudo chown root:wheel "$INSTALL_PATH"

# 设置 setuid 位 (4755 = rwsr-xr-x)
sudo chmod 4755 "$INSTALL_PATH"

echo ""
echo "步骤 2: 验证安装..."

# 验证权限
PERMS=$(ls -l "$INSTALL_PATH" | awk '{print $1}')
OWNER=$(ls -l "$INSTALL_PATH" | awk '{print $3":"$4}')

echo "文件权限: $PERMS"
echo "文件所有者: $OWNER"

if [[ "$PERMS" == "-rwsr-xr-x" ]] && [[ "$OWNER" == "root:wheel" ]]; then
    echo ""
    echo "=========================================="
    echo "  ✅ 安装成功！"
    echo "=========================================="
    echo ""
    echo "helper 已安装到: $INSTALL_PATH"
    echo "现在可以正常使用 anyFAST 修改 hosts 文件了"
    echo ""
    echo "请重启 anyFAST 应用"
else
    echo ""
    echo "=========================================="
    echo "  ⚠️ 安装可能不完整"
    echo "=========================================="
    echo ""
    echo "请手动检查并执行以下命令:"
    echo "  sudo chown root:wheel $INSTALL_PATH"
    echo "  sudo chmod 4755 $INSTALL_PATH"
fi
