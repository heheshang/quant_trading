# Harness Engineering 体系

> 围绕 AI Coding Agent 设计和构建约束机制、反馈回路、工作流控制与持续改进循环的系统工程实践。

## 核心原则

**每发现一个 Agent 的错误，就工程化地消除它再次发生的可能性。**
— Mitchell Hashimoto（HashiCorp 创始人）

## 四要素架构

| 要素 | 路径 | 职责 | 更新频率 |
|------|------|------|----------|
| **规则体系 Rules** | `.harness/rules/` | 告诉 Agent "标准是什么" — 工程结构约束、编码规范 | 按需，每次发现新约束 |
| **技能体系 Skills** | `.harness/skills/` | 告诉 Agent "应该怎么做" — 需求分析 SOP、编码分层规范 | 每次流程优化后 |
| **知识库 Wiki** | `.harness/wiki/` | 告诉 Agent "系统是什么样的" — 业务链路、数据模型 | 随项目演进持续更新 |
| **变更管理 Changes** | `.harness/changes/` | 记录 Agent "做了什么" — 每个需求的完整追溯链 | 每个需求新建 |

## 目录结构

```
.harness/
├── README.md                    # 本文件 — 体系总览（含 L1/L2/L3）
├── init.md                      # 引导初始化 + Dry Run 流程
├── platform.md                  # 跨平台适配（Codex / Claude Code）
├── agents/
│   ├── initializer-agent.md     # 初始化 Agent（首次运行环境搭建）
│   ├── owner-agent.md           # 编排中枢（流程调度 + 阶段定义）
│   ├── planner-agent.md         # 规划 Agent（需求分析 + 任务拆解）
│   ├── generator-agent.md       # 执行 Agent（编码 + 测试）
│   └── evaluator-agent.md       # 评判 Agent（评审 + 质量门禁）
├── rules/
│   ├── arch-rules-rust.md       # 架构规则与约束
│   ├── coding-rules-rust.md     # 编码规范
│   ├── quality-gates.md         # 8 个可程序化验证的质量门禁
│   ├── workflow-rules.md        # 流程规则（非阶段定义）
│   ├── entropy-gc.md            # 熵清理流程（后台偏差扫描）
│   └── linter-examples-rust.md  # 10 条机械执行 Lint 规则（LINT-001~010）
├── skills/
│   ├── request-analysis/        # 需求分析 SOP
│   ├── coding-skill/            # 编码实现（含 7 份 Rust 分层 Spec）
│   ├── expert-reviewer/         # 评审方法 + 报告模板
│   ├── unit-test-write/         # 改动驱动测试
│   ├── unit-test-ci/            # CI 流水线验证
│   └── deploy-verify/           # 部署验证
├── wiki/                        # 项目知识库
├── scripts/
│   ├── verify-qg.sh             # 门禁 Shell 验证脚本
│   └── verify-qg.py             # 门禁 Python 验证脚本
├── changes/
│   └── template/                # 变更目录模板
└── mcp/mcp-config.md            # MCP 工具集成
```

## 上下文分层加载策略（L1/L2/L3）

### L1 — 会话常驻层（Always Loaded）

每次会话启动时自动加载，提供全局视野和基本约束。

| 文件 | 行数 | 说明 |
|------|------|------|
| `.harness/agents/owner-agent.md` | ~550 行 | 编排中枢 + 会话启动仪式 + PR 工作流 |
| `.harness/rules/arch-rules-rust.md` | ~250 行 | 架构约束 |
| `.harness/rules/coding-rules-rust.md` | ~210 行 | 编码规范 |
| `.harness/rules/workflow-rules.md` | ~100 行 | 流程规则 |

**总 Token 预算**：~3,000（~10% 上下文窗口）

### L2 — 阶段触发层（Phase-triggered）

进入特定阶段时触发加载，提供当前阶段所需的专业知识。

| 阶段 | 加载文件 |
|------|----------|
| 需求分析（阶段 1） | `.harness/skills/request-analysis/skill.md` |
| 需求评审（阶段 2） | `.harness/skills/expert-reviewer/skill.md` |
| 编码实现（阶段 3） | `.harness/skills/coding-skill/*.md`（按需加载分层 Spec） |
| 编码评审（阶段 4） | `.harness/skills/expert-reviewer/skill.md` |
| 测试编写（阶段 5） | `.harness/skills/unit-test-write/skill.md` |
| CI 验证（阶段 6）  | `.harness/skills/unit-test-ci/skill.md` |

**总 Token 预算**：~2,000（~7% 上下文窗口）

### L3 — 按需查询层（On-demand）

Wiki 知识库中的业务文档不会主动加载，Agent 根据任务需要自主查阅。

| 文件 | 查询时机 |
|------|----------|
| `.harness/wiki/context-overview.md` | 需要了解项目整体结构时 |
| `.harness/wiki/data-model.md` | 涉及数据库变更时 |
| `.harness/wiki/biz-flows.md` | 涉及核心业务流程变更时 |

**总 Token 预算**：~1,500（~5% 上下文窗口）

### 上下文窗口预算

遵循 Anthropic 经验：**上下文窗口填充率不超过 40%**。超出时优先卸载 L3，其次 L2，L1 不可卸载。

### 会话初始化序列

> **单一事实来源：** `agents/owner-agent.md` 第零节（会话启动仪式）。
> 此处仅作概要引用，完整步骤以 owner-agent.md 为准。

```
1. 执行 owner-agent.md 第零节的 8 步启动仪式（唯一权威版本）
   → pwd → init.sh → git log → feature_list.json → summary.md → detect-build.sh + compile
2. 读取 AGENTS.md（项目根目录）→ 指向 .harness/
3. 读取 .harness/platform.md → 确定平台适配
4. 读取 .harness/README.md → 理解体系（本文件）
5. 读取 .harness/rules/（L1）
6. 开始工作
```

## 持续集成（CI）

> 仓库已集成 GitHub Actions CI（`.github/workflows/harness-ci.yml`），
> 在 push/PR 到 main 时自动执行：

```yaml
Job 1 (Init): 运行 init.sh + detect-build.sh → 验证构建工具检测
Job 2 (QG):   运行 QG-1~4 质量门禁（reporting mode，不阻塞）
Job 3 (Lint):  ShellCheck + Python 语法检查
Job 4 (Docs):  验证关键文件完整性 + Markdown 链接检查
```

> **从 Reporting 切换到 Strict 模式**：移除 CI workflow 中的 `continue-on-error: true`，
> 质量门禁失败将直接阻止 PR 合并。

## 核心 / 扩展阶段流

| 范围 | 阶段 | 说明 |
|------|------|------|
| **Core** | 1→6 | Agent 可在单次会话中独立完成 |
| **Extended** | 7→10 | 涉及部署/灰度，需跨 Session + 人工 |

```
Core (Agent 独立完成):
  需求分析 → 需求评审 → 编码实现 → 编码评审 → 单元测试编写 → CI 验证
Extended (跨 Session + 人工):
  集成测试 → 部署验证 → 灰度发布 → 交付确认
```

## 快速使用

```yaml
# 首次使用
1. 读取 AGENTS.md → 理解 Harness 入口
2. 读取 .harness/init.md → 执行 Dry Run 验证

# 日常开发
1. 接到新需求 → cp -r .harness/changes/template .harness/changes/feat-xxx-$(date +%Y%m%d)
2. 读取 owner-agent.md → 按 6 阶段流程推进
3. 每个阶段完成 → 更新 summary.md
4. 核心 6 阶段完成 → 人工确认 → 进入扩展阶段

# 持续改进
1. 发现 Agent 错误 → 更新相应的 Rules 或 Skills
2. 运行 Dry Run 验证修复有效
3. 如果修复涉及流程变更 → 更新 workflow-rules.md
```

## 参考资料

- [Anthropic: Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- [Anthropic: Harness design for long-running application development](https://www.anthropic.com/engineering/harness-design-long-running-apps)
- [OpenAI: Harness engineering — leveraging Codex in an agent-first world](https://openai.com/index/harness-engineering/)
