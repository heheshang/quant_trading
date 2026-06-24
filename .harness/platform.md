# 平台适配层

> **关键文件**：确定当前运行平台，将 Harness 指令映射为平台具体操作。
> 任何阶段开始前必须读取本文件。

## 平台检测

Agent 启动时通过 `detect-platform.sh` 自动检测当前平台。逻辑等价于：

```bash
source .harness/scripts/detect-platform.sh
# 导出: PLATFORM, PLATFORM_CONFIDENCE, MCP_CONFIG_PATH, CI_AVAILABLE, ...
```

## 工具映射

| Harness 操作 | Claude Code | OpenAI Codex CLI | OpenCode |
|-------------|-------------|------------------|----------|
| 读取文件 | `Read` 工具 | `read` 或 `cat` | `Read` 工具 |
| 写入文件 | `Write` / `Edit` 工具 | `write` / `edit` | `Write` / `Edit` |
| 执行命令 | `Bash` 工具 | `bash` 或 `shell` | `Bash` / `interactive_bash` |
| 运行测试 | `Bash: source detect-build && ${TEST_CMD}` | `bash: source detect-build && ${TEST_CMD}` | `Bash: source detect-build && ${TEST_CMD}` |
| 编译检查 | `Bash: source detect-build && ${BUILD_CHECK_CMD}` | `bash: source detect-build && ${BUILD_CHECK_CMD}` | `Bash: source detect-build && ${BUILD_CHECK_CMD}` |
| 文件搜索 | `Grep` / `Glob` 工具 | `grep` / `find` | `Grep` / `Glob` |
| MCP 工具 | `skill_mcp` 工具 | 通过 STDIO MCP | `skill_mcp` 工具 |
| 截图验证 | Puppeteer MCP | Playwright MCP | Playwright MCP |
| CI 查询 | CI MCP | CI MCP | CI MCP |
| 数据库查询 | Database MCP | Database MCP | Database MCP |
| 日志查询 | Log MCP | Log MCP | Log MCP |

### MCP 配置差异

```yaml
# Claude Code MCP 配置 (~/.claude/settings.json)
{
  "mcpServers": {
    "puppeteer": {
      "command": "npx", "args": ["-y", "@anthropic-ai/puppeteer-mcp"]
    }
  }
}

# Codex CLI MCP 配置 (~/.codex/mcp.json)
{
  "mcpServers": {
    "playwright": {
      "command": "npx", "args": ["-y", "@playwright/mcp"]
    }
  }
}

# OpenCode MCP 配置 (~/.config/opencode/opencode.json 或项目 opencode.jsonc)
# MCP 服务器通过 opencode.jsonc 的 mcpServers 字段定义
# 或通过 MCP 协议自动发现（STDIO / HTTP）
```

## 平台适配原则

### 文件操作
- **所有平台**都支持直接文件读写，**不需要区分**
- 优先使用平台原生读写工具（Read/Write/Edit/Grep/Glob），效率更高
- 只在原生工具不可用时才 fallback 到 shell 命令

### 命令执行
- 构建/测试命令由 detect-build.sh 自动适配，在**所有平台**执行方式一致
- git 命令完全一致

### MCP 工具
- Claude Code：使用 `skill_mcp` 工具调用 MCP
- Codex CLI：通过内置 MCP Client 自动调用
- OpenCode：使用 `skill_mcp` 工具或 MCP Client 自动调用
- 如果 MCP 工具不可用，降级为 shell 命令

### 截图验证
- Claude Code：Puppeteer MCP（`@anthropic-ai/puppeteer-mcp`）
- Codex CLI / OpenCode：Playwright MCP（`@playwright/mcp`）
- 功能等价，只是包名不同

## 会话初始化序列

> 完整启动仪式定义在 `agents/owner-agent.md` 第零节。
> 此处只列出平台相关的步骤。**请勿独立定义一套新的序列。**

```yaml
1. 执行 owner-agent.md 第零节的 8 步启动仪式（不区分平台）
2. 加载本文件（platform.md）→ 确定平台工具适配
3. 读取 AGENTS.md 或 CLAUDE.md（项目根目录）
4. 读取 .harness/README.md → 理解体系概览
5. 读取 .harness/rules/ → 加载 L1 约束
6. 开始工作
```

## 已知平台差异

| 差异点 | Claude Code | Codex CLI | OpenCode |
|--------|-------------|-----------|----------|
| MCP 配置位置 | `~/.claude/settings.json` | `~/.codex/mcp.json` | `~/.config/opencode/opencode.json` |
| 项目配置 | `CLAUDE.md` | `.codex/` | `opencode.jsonc` + `.opencode/` |
| MCP 调用方式 | `skill_mcp` 工具 | MCP Client 自动 | `skill_mcp` / MCP Client |
| 文件读写工具 | Read/Write/Edit/Grep/Glob | read/write/edit | Read/Write/Edit/Grep/Glob |
| LSP 配置 | 内置 | `.codex/lsp-client.json` | `.opencode/lsp.json` |
| 环境变量 | `CLAUDE_CODE=1` | `CODEX_CLI=1` | `OPENCODE=1`（部分终端） |
