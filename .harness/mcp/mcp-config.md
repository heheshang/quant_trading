# MCP 工具集成配置

> MCP（Model Context Protocol）外部工具集成。用于扩展 Agent 的能力边界。
> 文章强调：Anthropic 的解决方案是引入 Browser Automation（Puppeteer MCP）进行自动化的端到端验证截图。

## MCP 工具清单

| 工具 | 用途 | 使用阶段 | 必需/可选 |
|------|------|----------|-----------|
| Puppeteer/Playwright | 浏览器自动化，端到端验证截图 | 编码评审、集成测试 | **必需** |
| Database MCP | 数据库查询和操作 | 编码实现、数据验证 | 推荐 |
| CI MCP | CI pipeline 触发和结果查询 | 单元测试 CI | 推荐 |
| Log MCP | 日志查询和分析 | 部署验证、问题排查 | 可选 |
| Monitor MCP | 监控指标查询 | 部署验证、灰度发布 | 可选 |

## 配置方式

### 1. Browser MCP（端到端验证）

**Claude Code（Puppeteer MCP）：**
```json
{
  "mcpServers": {
    "puppeteer": {
      "command": "npx",
      "args": ["-y", "@anthropic-ai/puppeteer-mcp"]
    }
  }
}
```

**Codex CLI（Playwright MCP）：**
```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["-y", "@playwright/mcp"]
    }
  }
}

**OpenCode（Playwright MCP）：**
OpenCode 支持两种 MCP 配置方式：
- 全局：`~/.config/opencode/opencode.json` 的 `mcpServers` 字段
- 项目级：`opencode.jsonc` 的 `mcpServers` 字段
- 自动发现：通过 STDIO 或 HTTP 协议自动连接已运行的 MCP 服务器

MCP 调用统一使用 `skill_mcp` 工具。

```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["-y", "@playwright/mcp"]
    }
  }
}
```

**用途 — 文章明确要求：**
- 编码评审阶段：自动截图验证 UI 变更，作为评审证据
- 集成测试阶段：验证端到端流程是否正常
- 部署验证阶段：检查页面可访问性和响应

**使用示例：**
```
# 截图验证接口返回
mcp__puppeteer__navigate("http://localhost:8080/api/v1/price/query?itemId=12345")
mcp__puppeteer__screenshot("${CHANGE_DIR}/verify-screenshot.png")

# 验证页面元素
mcp__puppeteer__evaluate("document.querySelector('.price-value').textContent")
```

### 2. Database MCP

> ⚠️ 以下 MCP 配置为示例用法。`@anthropic-ai/database-mcp` 等包名是占位符，
> 实际部署时需要替换为真实可用的 MCP 包或自定义实现。
> 详情参见 [platform.md](../platform.md) 的平台差异说明。

```json
{
  "mcpServers": {
    "database": {
      "command": "npx",
      "args": ["-y", "@anthropic-ai/database-mcp", "--url", "${DB_URL}"]
    }
  }
}
```

**用途：**
- 编码阶段：验证数据模型变更
- 测试阶段：查询线上真实请求构造测试数据
- 验证阶段：确认数据变更是否符合预期

### 3. CI MCP

```json
{
  "mcpServers": {
    "ci": {
      "command": "npx",
      "args": ["-y", "@anthropic-ai/ci-mcp"]
    }
  }
}
```

**用途：**
- 触发 CI pipeline
- 查询 CI 执行状态和结果
- 获取测试报告和覆盖率数据

### 4. Log MCP

```json
{
  "mcpServers": {
    "log": {
      "command": "npx",
      "args": ["-y", "@anthropic-ai/log-mcp"]
    }
  }
}
```

**用途：**
- 部署验证阶段：检查服务启动日志
- 问题排查：查询错误日志定位问题

### 5. Monitor MCP

```json
{
  "mcpServers": {
    "monitor": {
      "command": "npx",
      "args": ["-y", "@anthropic-ai/monitor-mcp"]
    }
  }
}
```

**用途：**
- 部署验证：检查 CPU/内存/GC 指标
- 灰度发布：监控错误率和 RT 变化

## 集成点

### 阶段 3（编码实现）
- Database MCP：查询线上真实请求构造测试数据
- Puppeteer MCP：验证 UI 变更截图

### 阶段 4（编码评审）
- Puppeteer MCP：截图验证接口返回值
- Database MCP：验证数据变更

### 阶段 5（单元测试编写）
- Database MCP：查询线上数据构造测试用例

### 阶段 6（单元测试 CI）
- CI MCP：触发和监控 CI 流水线

### 阶段 7（集成测试）
- Puppeteer MCP：端到端流程验证截图
- Database MCP：验证数据一致性

### 阶段 8（部署验证）
- Monitor MCP：检查服务健康指标
- Log MCP：检查启动日志

### 阶段 9（灰度发布）
- Monitor MCP：监控业务指标
- Log MCP：监控错误日志

## 使用原则

1. **最小权限**：MCP 工具使用场景受限，仅在需要的阶段启用
2. **数据安全**：数据库 MCP 只读权限优先，写操作需显式确认
3. **容错处理**：MCP 工具不可用时应有降级方案
4. **审计日志**：MCP 工具的每次调用结果应记录在变更文档中
5. **截图存档**：所有 Puppeteer 截图保存到变更目录，作为评审证据
