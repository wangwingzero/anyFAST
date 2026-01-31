#!/bin/bash
# anyFAST macOS Helper 卸载脚本

HELPER_NAME="anyfast-helper-macos"
INSTALL_PATH="/usr/local/bin/$HELPER_NAME"

echo "=========================================="
echo "  anyFAST macOS Helper 卸载程序"
echo "=========================================="
echo ""

if [[ ! -f "$INSTALL_PATH" ]]; then
    echo "helper 未安装或已被删除"
    exit 0
fi

echo "正在删除 $INSTALL_PATH..."
sudo rm -f "$INSTALL_PATH"

if [[ ! -f "$INSTALL_PATH" ]]; then
    echo ""
    echo "✅ 卸载成功！"
else
    echo ""
    echo "⚠️ 卸载失败，请手动删除: sudo rm $INSTALL_PATH"
fi
