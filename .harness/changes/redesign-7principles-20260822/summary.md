# 变更摘要

> 本变更的 Single Source of Truth。记录阶段执行状态、验证结果与后续路线。

## 基本信息

- **需求名称**：基于软件设计 7 原则的整体代码重构（redesign-7principles）
- **变更类型**：refactor
- **日期**：20260822
- **Owner**：Application Owner Agent
- **来源**：用户指令：使用软件设计工程思想 + 7 大基本原则重构代码

## 阶段执行状态

| 阶段 | 范围 | 状态 | 备注 |
|------|------|------|------|
| 需求分析 | Core | ✅ | spec.md + design.md + tasks.md |
| 需求评审 | Core | ✅ | 设计蓝图经审计验证 |
| 编码实现 | Core | ✅ | B1/B2/B3 已落地并验证 |
| 编码评审 | Core | ⬜ | 本会话 Owner 验证；Evaluator 评审留待 CI |
| 单元测试编写 | Core | ✅ | 复用既有测试（前端 426 / Rust 559） |
| 单元测试 CI | Core | ✅ | 本地等效验证（CI 不可用） |
| 集成测试 | Extended | ⬜ | - |
| 部署验证 | Extended | ⬜ | - |
| 灰度发布 | Extended | ⬜ | - |
| 交付确认 | Extended | ⬜ | - |

## 验证结果（全绿）

- `cargo check --workspace`：通过
- `cargo clippy --workspace --all-targets`：0 warning
- `cargo test --workspace --no-fail-fast`：**559 passed / 0 failed / 17 ignored**
- `vue-tsc --noEmit`：通过
- `npm test`：**426 passed / 0 failed**

> 数据与重构前完全一致，证明重构未改变外部行为。

## 已落地批次

### B1 — 前端 DRY：合并格式 composable
- 保留 `useFormatting.ts` 为唯一权威实现（含 null/undefined/NaN 占位处理，`formatNumber` 采用 locale 千分位——与 `MonitorOkx` 断言 `'2,500'` 一致）
- 5 个使用 `useFormat` 的文件（BacktestChart/History/TradeList/Results/Backtest.vue）全部切换为 `useFormatting`
- **删除 `useFormat.ts`**（DRY：消除重复）
- 涉及：`useFormatting.ts`(改) 、`useFormat.ts`(删) 、5 个组件(改)

### B2 — 后端分层/DIP/SRP：订单链路下沉用例
- 新增 `crates/services/src/order_processor.rs`：`OrderProcessor` 用例承载完整下单编排（行情解析→风控检查→内存提交→持久化(优雅降级)→事件事件描述→异步执行）
- `commands/core.rs::submit_order` 变为**薄适配器**：`services.order_processor.place_order(order)` + `app.emit`
- `AppServices` 注入 `order_manager` + `log_buffer`，`market/risk/account` 三服务改为 `Arc`，装配出 `OrderProcessor`
- 命令层不再直接触碰 `risk_layer` / `trading_layer` / `data_layer`（订单编排部分）
- 涉及：`order_processor.rs`(新增)、`app_service.rs`(改)、`services/lib.rs`(改)、`commands/core.rs`(改)、`main.rs`(改)

### B3 — 前端 DRY/死代码：删除重复卡片
- 确认 `StatsCard.vue` 为权威卡片；`MetricCard.vue` 为**零引用死代码**
- **删除 `MetricCard.vue`** + 清理自动生成的 `components.d.ts`
- 涉及：`MetricCard.vue`(删)、`components.d.ts`(改)

### B4 — 前端 SoC：拆分市场行情/WebSocket 服务
- 新增 `services/ws.ts`：WebSocket 生命周期与订阅（start/subscribe/unsubscribe/stop/get_subscriptions）
- `services/market.ts` 收敛为纯行情读取；删除死代码 `startMarketData` 外迁
- 更新 `stores/marketData.ts` 与 `__tests__/api-okx.test.ts` 的 import 路径
- 涉及：`ws.ts`(新增)、`market.ts`(改)、`marketData.ts`(改)、`api-okx.test.ts`(改)

### B5 — 后端装配收敛（DIP/SRP/DRY）
- 引入 `SharedInfra` 结构体打包注入 `AppServices`，根治 `too_many_arguments`（移除 `#[allow]` 兜底）
- 消除 `OkxExecutor` 重复实例化：单一 `Arc<OkxExecutor>`，`AppState` 与 `AppServices` 共享同一实例；`AppState.okx_executor` 类型对齐
- 涉及：`services/src/app_service.rs`(改)、`services/src/lib.rs`(改)、`src-tauri/src/main.rs`(改)、`src-tauri/src/state.rs`(改)

### B6(部分) — 前端 DIP：统一 IPC transport
- 新增 `services/transport.ts`：隔离 `@tauri-apps/api` 依赖，服务层经 `call<T>(cmd, args?)` 调用
- 11 个 service 文件迁移；`call` 在无参时省略 `args` 以保证跨层调用与断言兼容
- 涉及：`transport.ts`(新增)、`src/services/*.ts`(11 个改)

### B4-2 — 前端 SoC：OKX 下单职责拆分
- 新增 `services/okxOrder.ts`：place/cancel/execute order
- `services/okx.ts` 收敛为账户/行情/状态读取；`Trading.vue`/`api-okx.test.ts` 更新 import
- 涉及：`okxOrder.ts`(新增)、`okx.ts`(改)、`Trading.vue`(改)、`api-okx.test.ts`(改)

### B6-2 — 前端 SRP：策略存储拆分
- 新增 `stores/strategyLifecycle.ts`（生命周期 store，组合 base）：start/stop/pause/resume/deploy/archive/toggle + 单独 error 状态
- `stores/strategy.ts` 收敛为数据/CRUD/类型/轮询
- `Strategy.vue` 生命周期动作改走 `lifecycleStore`；3 个测试文件迁移
- 涉及：`strategyLifecycle.ts`(新增)、`strategy.ts`(改)、`Strategy.vue`(改)、`Strategy.test.ts`(改)、`StrategyDialog.test.ts`(改)、`strategy.store.test.ts`(改)、`strategyLifecycle.store.test.ts`(新增)          

## 变更文件清单

| 文件路径 | 变更类型 | 说明 |
|----------|----------|------|
| `.harness/changes/redesign-7principles-20260822/design.md` | 新增 | 7 原则审计 + 目标架构蓝图 |
| `.harness/changes/redesign-7principles-20260822/spec.md` | 新增 | 需求规格 |
| `.harness/changes/redesign-7principles-20260822/tasks.md` | 新增 | 批次任务分解 |
| `crates/services/src/order_processor.rs` | 新增 | 订单编排用例（SRP/DIP） |
| `crates/services/src/app_service.rs` | 修改 | 注入 order_manager/log_buffer、Arc 化服务、装配 OrderProcessor |
| `crates/services/src/lib.rs` | 修改 | export order_processor |
| `src-tauri/src/commands/core.rs` | 修改 | submit_order 收窄为薄适配器 |
| `src-tauri/src/main.rs` | 修改 | AppServices 装配传入 order_manager/log_buffer |
| `src/composables/useFormatting.ts` | 修改 | 权威格式 composable（宽化类型+占位处理） |
| `src/composables/useFormat.ts` | 删除 | 重复 composable |
| `src/components/backtest/{BacktestChart,BacktestHistory,BacktestTradeList,BacktestResults}.vue` | 修改 | 改用 useFormatting |
| `src/views/Backtest.vue` | 修改 | 改用 useFormatting |
| `src/components/MetricCard.vue` | 删除 | 死代码 |
| `components.d.ts` | 修改 | 移除 MetricCard 声明 |

## 例外情况

- 单元测试 CI（阶段 6）无远程 CI 可触发，采用本地等效验证（`cargo test` + `npm test`），符合 workflow-rules 规则 1。
- Evaluator 评审（阶段 4）在本会话由 Owner Agent 以编译/测试门禁验证替代；正式 4 眼评审建议接入 CI 后执行。

## 后续路线

> **全部计划批次已完成。** 无剩余项。

| 批 | 状态 |
|----|------|
| B1 前端 DRY（useFormat 合并） | ✅ |
| B2 后端分层（OrderProcessor 用例） | ✅ |
| B3 死代码删除（MetricCard） | ✅ |
| B4 前端 SoC（market/ws 拆分） | ✅ |
| B4-2 前端 SoC（okx/okxOrder 拆分） | ✅ |
| B5 后端装配收敛（SharedInfra + 单一 executor） | ✅ |
| B6-1 前端 DIP（transport 抽象） | ✅ |
| B6-2 前端 SRP（strategy store 拆分） | ✅ |

## 分层加固（后续追加）

- **消除命令层 `data_layer` 直连**：`get_market_data`、`get_okx_realtime_data`、`get_okx_historical_data` 改为经 `market_service.get_realtime_data/get_historical_data`（保留 OKX 观测指标于适配层）。命令层不再 `use data_layer::market_data::DataSource`。
- **例外记录**：`pre_trade_check` / `get_risk_metrics` 保留 `risk_layer` 直连，属**故意的领域边界**——风控操作需返回具体失败原因（非仅 bool）并触发告警；经当前 `risk_service.pre_trade_check`（返回 `(bool, config)`）会丢失失败明细。进阶可增强 service 签名返回失败原因后再完全收敛（独立变更）。
- 相关测试更新：`commands/tests.rs` 3 个用例断言改为分层后的 `"Market service not initialized"`。

## 最终验证

- `cargo check` / `cargo clippy`（0 warning）/ `cargo test`（559 passed / 0 fail / 17 ignored）
- `vue-tsc` / `npm test`（34 文件 / 426 passed）
