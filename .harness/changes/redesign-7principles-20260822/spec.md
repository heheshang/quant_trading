# 需求规格说明书

## 背景

现有 `quant-trading-system`（Rust + Tauri 2 + Vue 3，约 46K 行）在演进过程中出现了：
- **`src-tauri/src/commands/core.rs::submit_order`** 成为 ~110 行「上帝函数」，混合行情、风控、订单、持久化、事件、异步执行。
- **命令层绕过 `services` 层**直接实例化 `risk_layer` / `trading_layer` / `data_layer` / `monitor_layer`，破坏单向分层。
- **`AppState` 与 `AppServices` 重复持有基础设施 Arc**，职责混杂。
- 前端 **`useFormat.ts` 与 `useFormatting.ts` 高度重复**，`MetricCard.vue` 与 `StatsCard.vue` 重叠。
- **`main.rs`** 为 305 行巨型装配。

需求：贯彻**软件设计 7 原则**（SRP/OCP/LSP/ISP/DIP/DRY/KISS），在不改变外部行为的前提下，系统性收敛这些设计债。

## 需求描述

对整体代码库做一次**基于 7 原则的架构重构**：
1. 前端 DRY：合并重复的格式 composable 与卡片组件。
2. 后端分层/DIP/SRP：把订单链路下沉为 `services` 层的**用例**（OrderProcessor），命令层变为纯适配器。
3. 前端 SoC：拆分红合的 service（行情/ws、okx 下单/行情）。
4. 装配收敛：`AppState`/`AppServices` 依赖收敛，消除重复实例化。

## 变更范围

### 涉及的模块
- `src-tauri/src/commands/core.rs`：订单链路改为调用用例
- `src-tauri/src/state.rs`、`main.rs`：装配收敛
- `crates/services/`：新增 `order` 用例模块
- `src/composables/useFormat*.ts`：合并为单一 `useFormatting`
- `src/components/MetricCard.vue`、`StatsCard.vue`：合并为 `StatCard.vue`
- `src/services/market.ts`、`okx.ts`：拆出 `ws.ts`、`okxOrder.ts`
- `src/stores/strategy.ts`：拆分生命周期职责

### 涉及的代码层
- [x] Application / Command 层（src-tauri）
- [x] Service / 用例层（services）
- [ ] Domain
- [ ] Repository / sqlx
- [ ] Client / RPC
- [ ] Configuration
- [ ] Migrations

### 不涉及的模块
- **数据库 schema / 迁移**：重构不触碰数据模型（无 schema 变更）
- **领域类型**：`domain` 层保持零依赖，不做类型改动
- **业务行为**：不新增/删除/变更任何可观察功能

## 影响分析

| 影响维度 | 分析 | 风险等级 |
|----------|------|----------|
| 上下游兼容性 | 命令接口签名不变，前端 `invoke` 参数不变 | 低 |
| 数据兼容性 | 无 schema/字段变更 | 低 |
| 配置变更 | 无 | 低 |
| 国际化 | 前端文案逻辑合并不变，仅去重 | 低 |
| 性能 | 无额外 I/O；用例层为纯编排 | 低 |
| 安全 | 风控/鉴权逻辑保持等价 | 低 |

## 验收标准

### 正常场景
1. Given 编译环境, When `cargo check --workspace`, Then 通过且无新增 clippy 告警。
2. Given 前端已构建, When `vue-tsc --noEmit` 与 `npm test`, Then 全部通过（426 用例）。
3. Given 订单提交, When 调用 `submit_order`, Then 行为与重构前完全等价（风控、持久化、执行、事件均保留）。
4. Given 依赖图, When 检查命令层, Then 命令层不再直接依赖 `risk_layer`/`trading_layer`/`data_layer`/`monitor_layer`。

### 异常场景
1. Given 风控拦截, When 提交超限订单, Then 返回与重构前一致的错误。

### 边界条件
1. `useFormat.ts`（重复文件）被删除；全项目不存在对它的 import。

## 备注

- 依据 `arch-rules-rust.md`：单文件 ≤250 行；依赖单向；服务层是唯一编排点。
- 批次独立、可回滚，见 `design.md` 第 3、6 节。
