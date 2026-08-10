# 项目上下文总览

> 项目整体介绍、技术选型、模块职责。Agent 理解当前仓库的第一站。

## 项目简介

**项目名称**：quant-trading-system

**项目定位**：基于 Rust + Tauri 2 + Vue 3 的量化交易桌面应用，覆盖行情、策略、回测、
交易执行、风控和监控。当前仓库同时包含 `.harness/` 开发流程体系和业务源码。

## 技术栈

| 层 | 技术选型 | 说明 |
|----|----------|------|
| 桌面壳 | Tauri 2 | 前端 WebView + Rust 命令层 |
| 前端 | Vue 3 + Element Plus + ECharts + Pinia | 10 个页面视图 |
| 业务后端 | Rust workspace | 13 个业务 crates + `src-tauri` |
| 数据库 | PostgreSQL + sqlx 0.8 | 时序分区、迁移、CAS 更新 |
| 缓存 | Redis + deadpool-redis | 缓存与健康检查 |
| 交易所 | OKX SDK + reqwest + tokio-tungstenite | REST 与 WebSocket |
| 可观测性 | tracing + prometheus | 日志、指标、告警 |
| 测试 | Vitest + Cargo test + Mockall | 前端 423 个测试，Rust 各层单测 |

## 系统架构

```text
Vue3 前端 (src/)
  ├── views / components / stores / services / composables
  │        │ invoke / listen
  ▼        ▼
Tauri 后端 (src-tauri/)
  ├── commands.rs       业务命令
  ├── ws_commands.rs    行情 WebSocket 命令
  └── state.rs          共享 AppState
  │
  ▼
业务服务层 (crates/services)
  AppServices 装配 Auth / Account / Market / Strategy / Risk / Okx / Config
  │
  ├── 领域/策略/交易/风控/监控层
  └── 数据与外部依赖层 (data-layer / repository / clients / exchange-okx)
```

## Harness 开发流程

当前流程采用 Core + Extended 结构：

| 范围 | 阶段 | 说明 |
|------|------|------|
| Core 1-6 | 需求分析、需求评审、编码、编码评审、单测、CI 验证 | Agent 可独立完成 |
| Extended 7-10 | 集成测试、部署验证、灰度发布、交付确认 | 需要人工/DevOps |

关键约束：

- 阶段不可跳过，每个阶段有质量门禁。
- 编码前必须有 `spec.md` + `tasks.md`。
- 编码由 Generator Agent 完成，评审由 Evaluator Agent 完成。
- 变更记录放在 `.harness/changes/{type}-{name}-{date}/`。

## 模块职责

| 模块 | 路径 | 职责 |
|------|------|------|
| 公共层 | `crates/common` | 配置、公共类型、错误、工具 |
| 领域层 | `crates/domain` | 纯业务类型与状态机 |
| 数据层 | `crates/data-layer` | PostgreSQL、行情仓储、OKX 数据源 |
| 仓储层 | `crates/repository` | 策略/回测仓储、连接池 |
| 客户端 | `crates/clients` | Redis 缓存 |
| 服务层 | `crates/services` | 业务编排与装配 |
| 策略层 | `crates/strategy-layer` | 策略、指标、回测、调度 |
| 交易层 | `crates/trading-layer` | 订单、执行、算法单 |
| 风控层 | `crates/risk-layer` | 事前/实时/事后/VaR |
| 监控层 | `crates/monitor-layer` | 指标、日志、告警 |
| 安全层 | `crates/security` | 加密、认证、审计 |
| 交易所 | `crates/exchange-okx` | OKX REST/WS 客户端 |
| 拉取器 | `crates/data-puller` | 后台行情与快照拉取 |

## 环境

| 环境 | 用途 | 备注 |
|------|------|------|
| local | 本地开发 | `npm run tauri dev` |
| docker | 容器验证 | `compose.yaml` + `scripts/docker-test.sh` |
| CI | GitHub Actions | 当前仓库尚未包含 workflow，待补齐 |
