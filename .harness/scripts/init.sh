#!/bin/bash
# =============================================================================
# init.sh — Harness 会话启动脚本
# 每次 Agent 新会话开始时由 owner-agent.md 第零节调用。
# 幂等：多次运行结果相同。
#
# 由 Initializer Agent 生成（.harness/agents/initializer-agent.md）。
# =============================================================================

set -e

echo "🔧 Harness Session Init — $(date '+%Y-%m-%d %H:%M:%S')"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 0. 自动检测运行平台
source "${SCRIPT_DIR}/detect-platform.sh"

# 1. 自动检测构建工具并运行编译检查
source "${SCRIPT_DIR}/detect-build.sh"

# 1a. 运行编译检查
echo "  📦 Running: ${BUILD_CHECK_CMD}"
if ${BUILD_CHECK_CMD} 2>/dev/null; then
  echo "  ✅ ${BUILD_TOOL} compile OK"
else
  echo "  ❌ ${BUILD_TOOL} compile failed"
fi

# 2. 检查变更目录
if [ -d ".harness/changes" ]; then
  CHANGES=$(ls -d .harness/changes/*/ 2>/dev/null | wc -l | tr -d ' ')
  echo "  📂 Changes in progress: ${CHANGES}"
else
  echo "  📂 No .harness/changes directory yet"
fi

echo "✅ Harness init complete"
