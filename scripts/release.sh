#!/bin/bash
# 发版脚本：确保 tag 包含所有最新 commit
# 用法: ./scripts/release.sh v1.8.0 "release notes here"

set -e

TAG=$1
MESSAGE=$2

if [ -z "$TAG" ] || [ -z "$MESSAGE" ]; then
  echo "用法: ./scripts/release.sh <tag> <message>"
  echo "示例: ./scripts/release.sh v1.8.0 '新增XX功能'"
  exit 1
fi

# 确保在最新的 main 上
git fetch origin main
LOCAL=$(git rev-parse HEAD)
REMOTE=$(git rev-parse origin/main)

if [ "$LOCAL" != "$REMOTE" ]; then
  echo "⚠️  本地 HEAD 和 origin/main 不一致"
  echo "   本地:  $LOCAL"
  echo "   远程:  $REMOTE"
  echo "请先 git pull 或 git push 同步后再发版"
  exit 1
fi

# 显示自上次 tag 以来的所有 commit
LATEST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [ -n "$LATEST_TAG" ]; then
  echo "📋 自 $LATEST_TAG 以来的 commit:"
  git log --oneline "$LATEST_TAG"..HEAD
  echo ""
  COMMIT_COUNT=$(git rev-list --count "$LATEST_TAG"..HEAD)
  if [ "$COMMIT_COUNT" -eq 0 ]; then
    echo "⚠️  没有新 commit，无需发版"
    exit 1
  fi
fi

echo "🏷️  即将创建 tag: $TAG"
read -p "确认发版？(y/N) " confirm
if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
  echo "已取消"
  exit 0
fi

git tag -a "$TAG" -m "$MESSAGE"
git push origin "$TAG"
echo "✅ $TAG 已推送，GitHub Action 将自动构建发布"
