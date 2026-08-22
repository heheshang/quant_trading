# 任务分解

> 按 `design.md` 第 3 节批次编排。每批结束后必须通过对应验证命令方可进入下一批。

## 里程碑总览

| 批 | 目标 | 验证命令 |
|----|------|----------|
| B1 | 前端 DRY：合并 `useFormat` → `useFormatting` | `vue-tsc` + `npm test` |
| B2 | 后端分层：订单链路下沉 `OrderProcessor` 用例 | `cargo check` + `cargo test` |
| B3 | 前端 DRY：合并卡片组件 → `StatCard` | `vue-tsc` + `npm test` |
| B4 | 前端 SoC：拆分 `market/okx` service | `vue-tsc` + `npm test` |
| B5 | 后端装配收敛：`AppState`/`AppServices` 依赖收敛 | `cargo check` |
| B6 | 前端 store 拆分 + transport 抽象 | `vue-tsc` + `npm test` |

## 任务清单

### B1: 前端 DRY — 合并格式 composable

- [x] B1-1 保留 `useFormatting.ts` 为唯一权威实现（含 formatCurrency/formatNumber/formatDate/formatPercentage/formatOrderStatus/formatStrategyType/formatOrderSide）
- [x] B1-2 全项目把 `useFormat` 引用改为 `useFormatting`
- [x] B1-3 删除 `useFormat.ts`
- [x] B1-4 `vue-tsc` + `npm test` 通过

### B2: 后端分层 — 订单链路下沉用例

- [x] B2-1 在 `crates/services` 新增 `order` 用例模块（`OrderProcessor`），承载：行情获取→风控检查→提交→持久化→事件/异步执行
- [x] B2-2 `commands/core.rs::submit_order` 改为薄壳：构造/获取用例 → 调用
- [x] B2-3 确保 `AppServices` 能装配出 `OrderProcessor`（注入依赖）
- [x] B2-4 `cargo check` + `cargo test` 通过

### B3: 前端 DRY — 合并卡片组件

- [x] B3-1 确认 `StatsCard.vue` 为权威卡片；`MetricCard.vue` 为死代码
- [x] B3-2 删除死代码 `MetricCard.vue`（零引用）+ 清理 `components.d.ts`
- [x] B3-3 `vue-tsc` + `npm test` 通过

### B4: 前端 SoC — 拆分 service

- [x] B4-1 拆出 `services/ws.ts`（start/subscribe/unsubscribe/stop/get_subscriptions）
- [x] B4-2 拆出 `services/okxOrder.ts`（place/cancel/execute order）；`okx.ts` 收敛；`Trading.vue`/`api-okx.test.ts` 更新
- [x] `vue-tsc` + `npm test` 通过
- [x] B4-3 `market.ts` 瘦身，更新引用
- [x] B4-4 `vue-tsc` + `npm test` 通过

### B5: 后端装配收敛

- [x] B5-1 引入 `SharedInfra` 打包注入（根治 too_many_arguments，移除 #[allow]）
- [x] B5-2 消除 `OkxExecutor` 重复实例化（单一实例，`AppState`/`AppServices` 共享）
- [x] B5-3 `cargo check` + `cargo clippy` + `cargo test` 通过

### B6: 前端 store 拆分 + transport 抽象

- [x] B6-1 抽 `services/transport.ts` 统一 `invoke`（DIP：隔离 Tauri 依赖）
- [x] B6-2 `stores/strategy.ts` 拆分生命周期职责 → 新增 `stores/strategyLifecycle.ts`（组合 base）；`Strategy.vue` 与 3 个测试文件迁移
- [x] B6-3 `vue-tsc` + `npm test` 通过（34 文件 / 426 用例）
