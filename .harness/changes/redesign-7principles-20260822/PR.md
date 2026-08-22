## 概述
对 `quant-trading-system` 进行的一次**架构重构**：遵循软件设计 7 原则（SRP / OCP / LSP / ISP / DIP / DRY / KISS），在不改变外部行为的前提下收敛设计债。净代码 **−494 行**，测试基线完全一致。

## 改动要点

### 后端（Rust）
- **命令层薄壳**：新增 `OrderProcessor` 用例承载完整下单编排（行情→风控→提交→持久化→事件→异步执行），`submit_order` 从 ~110 行「上帝函数」变为薄适配器。
- **装配收敛**：引入 `SharedInfra` 打包注入，`AppServices` 构造参数由 10 → 2；消除 `OkxExecutor` 重复实例化（命令层与服务层共享同一 `Arc`）。
- **分层加固**：`get_market_data` / `get_okx_realtime_data` / `get_okx_historical_data` 改经 `market_service`，命令层**零 `data_layer` 直连**。

### 前端（Vue/TS）
- **DRY**：合并 `useFormat` → 权威 `useFormatting`；删除死代码 `MetricCard.vue`。
- **SoC**：拆 `services/market`+`ws`、`services/okx`+`okxOrder`。
- **SRP**：`stores/strategy` 拆分为 `strategy`（数据/CRUD/类型/轮询）+ `strategyLifecycle`（生命周期动作，组合 base）。
- **DIP**：新增 `transport.ts`，服务层不再直接 `import @tauri-apps/api`，框架依赖收敛到单点。

## 测试
| 检查 | 结果 |
|------|------|
| `cargo check --workspace` | ✅ |
| `cargo clippy --all-targets` | ✅ 0 warning |
| `cargo test --workspace` | ✅ 559 passed / 0 failed / 17 ignored |
| `vue-tsc --noEmit` | ✅ |
| `npm test` | ✅ 34 文件 / 426 passed / 0 failed |

## 已知例外（需评审知悉）
`pre_trade_check` / `get_risk_metrics` 保留 `risk_layer` 直连，为**有意风控领域边界**——需返回具体失败原因（非仅 `bool`）并触发告警；经当前 `risk_service.pre_trade_check`（返回 `(bool, config)`）会丢失失败明细，进阶需先增强 service 签名（建议独立变更）。

## 参考
- 设计蓝图：`.harness/changes/redesign-7principles-20260822/design.md`（含 7 原则审计 + AS-IS→TO-BE 架构图）
