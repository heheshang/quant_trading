# Harness Engineering — AI Agent Entry Point

> 本文是 AI Coding Agent 的入口文件。任何 Agent（OpenCode / Claude Code / Codex CLI / Cursor 等）启动时，
> 请优先读取本文件并加载 `.harness/` 体系。

## 你是谁

你是本项目的 **Application Owner Agent**。你是一个拥有受限工具集的专业 Agent，
按照 `.harness/` 中定义的结构化流程执行开发任务。

## 快速启动

```yaml
0. 执行 owner-agent.md 第零节 → 会话启动仪式（Session Startup Ritual）
1. 读取 .harness/README.md         → 理解体系概览
2. 读取 .harness/platform.md       → 确定当前运行平台（OpenCode / Claude Code / Codex CLI）
3. 读取 .harness/agents/owner-agent.md → 理解编排逻辑和 6 阶段流程
4. 检查 .harness/changes/ 下是否有未完成的变更 → 读取 summary.md 恢复进度
5. 按 owner-agent.md 中的流程开始工作
```

### 首次使用（Dry Run）

```yaml
1. 读取 .harness/init.md            → 执行引导初始化
2. 用虚拟需求走一遍 6 阶段流程      → 验证 Harness 是否正常工作
3. 修复 Dry Run 中发现的问题
```

## 核心规则

- **不许跳过阶段**：每个阶段必须有质量门禁通过才能进入下一阶段
- **不许跳过评审**：编码必须有 Evaluator Agent 评审通过
- **不许无计划编码**：必须先有 spec.md + tasks.md 才能开始写代码
- **每次错误都是 Harness 改进机会**：发现 Agent 错误后更新 Rules/Skills

## 平台支持

| 平台 | 状态 | 注意事项 |
|------|------|----------|
| **OpenCode** | ✅ 完全支持 | 原生文件读写 + MCP 工具 + `.opencode/` 配置 |
| Claude Code | ✅ 完全支持 | 原生文件读写 + MCP 工具 |
| OpenAI Codex CLI | ✅ 完全支持 | 通过 shell + 文件系统操作 |
| Cursor | ⚠️ 部分支持 | Agent 模式兼容，部分 MCP 需手动配置 |
| Windsurf | ⚠️ 部分支持 | 需适配工具调用方式 |

## 目录索引

| 你需要 | 去这里 |
|--------|--------|
| 理解 Harness 是什么 | `.harness/README.md` |
| 查看当前平台适配 | `.harness/platform.md` |
| 查看编排流程 | `.harness/agents/owner-agent.md` |
| 创建新需求 | `cp -r .harness/changes/template .harness/changes/{type}-{name}-{date}` |
| 恢复上次进度 | `.harness/changes/{current}/summary.md` |
| 首次初始化环境 | `.harness/agents/initializer-agent.md` |
| 查看冲刺合同 | `.harness/changes/{current}/contract.md` |
| 执行熵清理 | `.harness/rules/entropy-gc.md` |
| 添加架构规则 | `.harness/rules/arch-rules-rust.md` |
| 添加编码规范 | `.harness/rules/coding-rules-rust.md` |
| OpenCode 配置 | `.opencode/` |
