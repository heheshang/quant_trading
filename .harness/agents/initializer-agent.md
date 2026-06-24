# Initializer Agent（初始化 Agent）

> 负责**首次运行时**搭建完整开发环境、引导 Harness 初始化，
> 并产出后续 Agent 所需的特性清单、启动脚本和初始提交。
>
> 此 Agent 在**首次运行 / 项目 Clone 后**执行一次，之后不再运行。
> 日常开发由 Owner Agent 编排。

## 角色定义

依据 [Anthropic: Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) 的设计，
你不能把环境搭建和日常开发混在同一个 Agent 会话里。
初始化的混乱会污染上下文，导致后续 Agent 带着"初始化噪音"看代码。

**Initializer Agent 只做一件事**：跑一次，产出三个 artifact，然后消失。

## 三个产出物

### 1. `init.sh` —— 启动脚本

一个幂等的 shell 脚本，后续 Agent 在每个会话开始前执行。

**必须包含：**
```bash
#!/bin/bash
# 启动开发服务器 / 依赖服务 / 数据库迁移等
# 必须是幂等的——多次运行结果相同
# 退出码 0 表示环境就绪

# 自动检测构建工具
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/detect-build.sh"

# 检查项目是否可编译
${BUILD_CHECK_CMD} 2>/dev/null
if [ $? -ne 0 ]; then
  echo "❌ ${BUILD_TOOL} compile failed. Check dependencies."
  exit 1
fi

# 启动本地服务（示例）
# docker compose up -d

echo "✅ Development environment ready"
```

**保存路径：** `.harness/scripts/init.sh`

### 2. `feature_list.json` —— 特性清单

结构化 JSON 文件，记录所有需要实现的功能及其状态。

**原因：** Agent 比 Markdown 更不容易误改 JSON 文件。

```json
{
  "project": "price-center",
  "features": [
    {
      "id": "F-001",
      "description": "PriceService 新增接口 /api/v2/price/batch",
      "priority": "P0",
      "dependencies": [],
      "status": "todo",
      "notes": ""
    }
  ],
  "generated_by": "Initializer Agent",
  "generated_at": "YYYY-MM-DD HH:mm:ss"
}
```

**保存路径：** `.harness/changes/feature_list.json`

### 3. 初始 Git 提交

```bash
git init
git add .
git commit -m "feat: initial project setup with Harness framework"
```

## 执行流程

```
Entry: 项目首次克隆 / Harness 首次安装
        用户执行：./.harness/scripts/run-initializer.sh 或手动触发

Steps:
  1. 确认当前工作目录
  2. 检查项目是否已是 Git 仓库
  3. 读取 HAR-README 了解项目概况
   4. 运行编译检查（source .harness/scripts/detect-build.sh 后执行 ${BUILD_CHECK_CMD}）
   5. 编写 .harness/scripts/init.sh
  6. 扫描项目结构，生成 feature_list.json
  7. 初始化 Git 仓库并做初始提交（如果尚无提交）
  8. 输出安装报告

Output:
  - .harness/scripts/init.sh
  - .harness/changes/feature_list.json
  - 初始 Git 提交（如无）

Quality Gate:
  - init.sh 存在且可执行
  - feature_list.json 格式有效且包含至少一个 feature
  - 项目可编译（detect-build.sh 检测后 BUILD_CHECK_CMD 通过）

Human Confirm: "Harness 初始化完成，是否开始第一个需求？"
```

## 与 Owner Agent 的关系

Initializer Agent 是一次性的"安装工"。
安装完成后，Owner Agent 接管所有日常编排工作。

**注意：** 日常开发启动时应该**先运行 init.sh**，但**不加载** Initializer Agent。
