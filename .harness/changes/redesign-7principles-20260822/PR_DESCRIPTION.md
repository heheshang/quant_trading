# PR 描述 / 提交说明

> 面向 `main` 的一次**架构重构**（不改变任何外部行为）。采用 Conventional Commits，主类型 `refactor`。

---

## 提交信息（可复制）

```
refactor: apply SOLID + DRY/KISS + SoC across backend & frontend

Reorganize the codebase toward software-design principles without
changing observable behavior. Net ~494 lines removed.

Backend (Rust):
- Extract OrderProcessor use-case; submit_order becomes a thin adapter
  (SRP / layering / DIP)
- Introduce SharedInfra to bundle composition-root deps; single shared
  OkxExecutor instead of a duplicated instance (DIP / DRY)
- Route market-data commands (get_market_data, get_okx_*_data) through
  market_service so commands no longer touch data-layer directly

Frontend (Vue/TS):
- Merge useFormat -> canonical useFormatting; remove dead MetricCard.vue
  (DRY / YAGNI)
- Split services into market/ws and okx/okxOrder (Separation of Concerns)
- Split strategy store into strategy + strategyLifecycle (SRP)
- Add transport.ts to isolate the Tauri IPC dependency (DIP)

Verification (local, CI-equivalent):
- cargo check --workspace          pass
- cargo clippy --all-targets       0 warnings
- cargo test --workspace           559 passed / 0 failed / 17 ignored
- vue-tsc --noEmit                 pass
- npm test                         34 files / 426 passed / 0 failed
```

---

## 变更范围

| 区 | 类型 | 说明 |
|----|------|------|
| `crates/services/src/order_processor.rs` | 新增 | 订单编排用例（行情→风控→提交→持久化→事件→异步执行） |
| `crates/services/src/app_service.rs` | 修改 | `SharedInfra` 打包注入、Arc 化服务、装配 OrderProcessor |
| `crates/services/src/lib.rs` | 修改 | export `SharedInfra` / `order_processor` |
| `src-tauri/src/commands/{core,auth_okx}.rs` | 修改 | `submit_order` 薄壳；市场数据命令经 `market_service`（清理 `data_layer` 直连） |
| `src-tauri/src/commands/tests.rs` | 修改 | 3 用例同步分层后错误消息 |
| `src-tauri/src/main.rs` / `state.rs` | 修改 | `SharedInfra` 装配、单一 `OkxExecutor`、`AppState.okx_executor` 对齐 |
| `src/composables/useFormatting.ts` | 修改 | 权威格式 composable（wide 类型 + null/NaN 占位） |
| `src/composables/useFormat.ts` / `MetricCard.vue` | 删除 | 重复/死代码 |
| `src/services/{ws,okxOrder,transport}.ts` | 新增 | SoC 拆分 + IPC 抽象 |
| `src/services/{market,okx}.ts` | 修改 | 收敛为纯职责 |
| `src/stores/{strategy.ts,strategyLifecycle.ts}` | 修改/新增 | SRP store 拆分 |
| `src/views/{Strategy,Trading,Backtest}.vue` | 修改 | 生命周期 store 接入 / 服务 import 更新 |
| `src/__tests__/*` | 修改/新增 | 测试迁移（`strategyLifecycle.store.test.ts` 新增） |

> 完整设计见 `.harness/changes/redesign-7principles-20260822/design.md`。

---

## 设计原则落地（7 原则）

| 原则 | 落地 |
|------|------|
| **SRP** | OrderProcessor 用例、命令薄壳、market/ws/okxOrder 三服务分离、strategy/strategyLifecycle 双 store |
| **DIP** | SharedInfra 注入、call() 隔离 Tauri、命令不再直连 data_layer |
| **DRY** | useFormat 合并、transport 收敛样板、共享 `OkxExecutor` |
| **SoC** | 行情/WS/OKX订阅/下单边界清晰 |
| **YAGNI/KISS** | 删死代码，不做过度抽象（详见 design.md 第 4 节"不做什么"） |
| **OCP/LSP** | Rust trait 系统与既有抽象在编译期兑现，重构未破坏 |

---

## 测试与风险

- **外部行为零变化**：测试基线一致（Rust 559 / 前端 426）。仅新增 `strategyLifecycle.store.test.ts`。
- **已知例外**：`strategy_risk.rs` 的 2 处 `risk_layer` 直连（VaR、pre-trade check）为**有意的风控领域边界**——需保留具体失败原因与告警；经当前 `risk_service.pre_trade_check`（返回 `(bool, config)`）会丢失失败明细，故暂留，进阶需先增强 service 签名（独立变更）。
- **风险**：低。无数据库 schema / 迁移变更，无配置变更。
