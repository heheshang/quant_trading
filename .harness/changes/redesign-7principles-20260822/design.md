# 设计蓝图：基于软件设计 7 原则的整体重构

> 本文件是本次 `redesign-7principles` 变更的**目标架构设计**。
> 使用软件设计工程思想，系统性地把现有代码向 7 条基本原则收敛。
>
> 目标：**在不改变外部行为（无功能增删）的前提下，大幅降低耦合、消除重复、恢复分层，提升可测试性与可持续演进能力。**

---

## 0. 顶层的 7 条软件设计基本原则

本次重构所依据的 7 条基本原则：

| # | 原则 | 英文 | 一句话 |
|---|------|------|--------|
| 1 | 单一职责 | SRP | 一个模块只有一个变更原因 |
| 2 | 开闭 | OCP | 对扩展开放，对修改关闭 |
| 3 | 里氏替换 | LSP | 子类型必须可替换其父类型 |
| 4 | 接口隔离 | ISP | 客户端不应依赖它不使用的方法 |
| 5 | 依赖倒置 | DIP | 依赖抽象，不依赖具体 |
| 6 | 不重复 | DRY | 每份知识在系统中只有一个权威表示 |
| 7 | 保持简单 | KISS | 简单优先；辅助 `YAGNI`（不做不需要的）、`SoC`（关注点分离）、`LoD`（最少知识） |

> 7 条原则在本项目的**验证优先级**：`SRP` > `分层/DIP` > `ISP` > `DRY` > `KISS` > `OCP` > `LSP`。
> 其中 OCSP/LSP 主要由 Rust 的 trait 系统与既有抽象在编译期兑现，重构重点是 SRP、分层、DRY。

---

## 1. 现状审计（AS-IS）：违规点 → 原则映射

### 1.1 后端（Rust）

| 文件 / 模块 | 问题 | 违反原则 | 严重度 |
|-------------|------|----------|--------|
| `src-tauri/src/commands/core.rs::submit_order` | ~110 行「上帝函数」：行情获取、风控检查、订单提交、持久化、事件广播、异步执行混为一体 | SRP, 分层/DIP | 🔴 高 |
| `src-tauri/src/main.rs` | 305 行巨型装配函数，一次性完成 OKX/DB/Redis/DataPuller/AppServices/OrderManager 初始化 | SRP, SoC | 🔴 高 |
| `src-tauri/src/state.rs::AppState` | 同时持有基础设施（config/pg/redis/okx/order_manager/ws）与 `Option<AppServices>`，与 `AppServices` 内部重复持有大量 `Arc` | SRP, 内聚, DRY | 🟠 中 |
| `commands/*` 直接依赖 `risk_layer` / `trading_layer` / `data_layer` / `monitor_layer` | 命令层绕过 `services` 层直接调用领域/基础设施 | DIP, 分层 | 🔴 高 |
| `OkxExecutor` 持 `Arc<RwLock<dyn ClientInterface>>`；`main.rs` 重复构造两个 executor | 双重实例化、trait-object 加锁 | ISP, KISS | 🟠 中 |
| `commands/tests.rs`(776), `auth_okx.rs`(488), `strategy_risk.rs`(487) | 单文件过大，多职责 | SRP | 🟠 中 |
| 各层错误/格式化逻辑分散 | 无统一错误转换策略 | DRY | 🟡 低 |

### 1.2 前端（TS/Vue）

| 文件 / 模块 | 问题 | 违反原则 | 严重度 |
|-------------|------|----------|--------|
| `src/composables/useFormat.ts` vs `useFormatting.ts` | 4 个函数（formatCurrency/formatNumber/formatDate/formatPercentage）几乎重复 | DRY | 🔴 高 |
| `src/components/MetricCard.vue` vs `StatsCard.vue` | 高度重叠的「卡片」组件，仅差 loading/trend/click | DRY | 🟠 中 |
| `src/services/market.ts` | 行情数据 + WebSocket 订阅混在「一个 service」 | SRP, SoC | 🟠 中 |
| `src/services/okx.ts` | 下单 + 行情 + 公告混合 | SRP | 🟠 中 |
| `src/stores/strategy.ts` | 358 行，CRUD + 生命周期 + 轮询 + 每动作 loading/error 分片 | SRP | 🟡 低 |
| `src/services/*` 各文件顶部重复 `import { invoke }` 样板 | 无统一 transport 抽象 | DRY, DIP | 🟡 低 |

---

## 2. 目标架构（TO-BE）

### 2.1 分层依赖总则

```
┌──────────────────────────────────────────────────────────────┐
│  UI 层 (Vue3)  views/components/stores/composables            │
└──────────────┬───────────────────────────────────────────────┘
               │  invoke（thin transport）
┌──────────────▼───────────────────────────────────────────────┐
│  Application / Command 层 (src-tauri)                         │
│  · 命令 = 纯「适配器」：参数映射 + 调用用例 + 序列化            │
│  · 禁止内嵌业务编排                                          │
└──────────────┬───────────────────────────────────────────────┘
               │  use-case
┌──────────────▼───────────────────────────────────────────────┐
│  服务/用例层 (quant-services)                                  │
│  · AppServices 只做「装配」→ 暴露可观测的领域服务               │
│  · 新增 UseCase（如 OrderProcessor）承担跨服务编排             │
│  · 依赖：domain/repo/clients 抽象，不依赖具体实现              │
└──────┬──────────────┬───────────────┬────────────────────────┘
       │              │               │
┌──────▼─────┐ ┌──────▼─────┐  ┌──────▼──────┐
│ 领域层 domain│ │引擎层        │  │ 基础设施层     │
│ (纯类型/状态机)│ │ strategy/    │  │ repository/   │
│            │ │ trading/risk/│  │ clients/      │
│            │ │ monitor      │  │ data-layer/   │
│            │ │ (trait on top)│ │ exchange-okx/ │
└────────────┘ └────────────┘  └───────────────┘
```

**关键规则（从 arch-rules 强化）：**
- **依赖单向向下**：`command → services → {domain | engine | infra}`。
- **services 是唯一编排点**：命令层与 UI 层不允许直接触碰 engine/infra。
- **domain 零依赖**；基础设施依赖抽象（trait）而非具体。
- **每个 crate 在 `lib.rs` re-export 公共 API**，禁止单文件 > 250 行（QG-5）。

### 2.2 架构迁移总览（AS-IS → TO-BE）

#### AS-IS：命令层为“上帝层”，跨层直连（重构前）

```text
┌───────────────────────────────────────────────────────┐
│ Vue3 UI  views / components / stores / composables      │
└──────────────────────────┬────────────────────────────┘
                           │ invoke
┌──────────────────────────▼────────────────────────────┐
│ src-tauri/commands （上帝层）                            │
│  · submit_order ~110 行：行情/风控/提交/持久化/事件/执行 │
│  · 直接 new risk_layer::PreTradeRiskChecker             │
│  · 直接调用 trading_layer::ExecutionEngine              │
│  · 直接读 data_layer::OkxDataSource                     │
│  · AppState 同持 infra + AppServices（重复 Arc）         │
└────┬────────┬────────┬─────────┬───────────┬──────────┘
     ▼        ▼        ▼         ▼           ▼
 risk-layer  trading-  data-     monitor-   domain
             layer     layer     layer
```

#### TO-BE：命令=薄适配器，services=唯一编排点（重构后）

```text
┌───────────────────────────────────────────────────────┐
│ Vue3 UI  views / components / stores / composables      │
└──────────────────────────┬────────────────────────────┘
                           │ call<T>(cmd, args)  [transport.ts]
┌──────────────────────────▼────────────────────────────┐
│ command（薄壳）                                          │
│  · 参数映射 → 调用用例 → 序列化 / 事件                     │
│  · 零 data_layer 直连（经 market_service）               │
└──────────────────────────┬────────────────────────────┘
                           │ use-case
┌──────────────────────────▼────────────────────────────┐
│ services（唯一编排点）                                    │
│  · AppServices::assemble(SharedInfra)                  │
│  · OrderProcessor（下单用例）                            │
│  · MarketService / RiskService / OkxService / …        │
└────┬──────────┬──────────┬───────────────┬────────────┘
     ▼          ▼          ▼               ▼
 domain    engines(trait)  repository     data-layer
                             clients       exchange-okx
```

#### 前端模块拆分（SoC / SRP）

```text
src/services                    src/stores
  account.ts                      strategy.ts          (数据/CRUD/类型/轮询)
  auth.ts                         strategyLifecycle.ts (生命周期动作，组合 base)
  market.ts    (行情读取)          └─ useStrategyStore()
  ws.ts        (WS 生命周期/订阅)
  okx.ts       (账户/行情/状态)
  okxOrder.ts  (下单/撤单/执行)
  transport.ts (唯一 IPC 入口 call<T>)
  order.ts / risk.ts / config.ts / monitor.ts / backtest.ts
```

#### 装配收敛（DIP / DRY）

```text
main.rs
  └─ SharedInfra { config, postgres, redis, market_data,
                     okx_client, okx_executor(单个 Arc),
                     okx_data_source, order_manager, log_buffer }
       └─ AppServices::with_config_path(infra, path)
            └─ assemble() → 各 service + OrderProcessor

AppState.okx_executor  ──同─个─ Arc<OkxExecutor>── AppServices.okx_executor
```

### 2.3 目标模块拆分

#### A. 后端：命令层 → 用例层

| 现状 | 目标 |
|------|------|
| `commands/core.rs::submit_order` 内含全链路 | 新增 `services::order::OrderProcessor`（用例），命令只做 参数→用例→响应 |
| `AppState` 同时持有全部基础设施 + `AppServices` | `AppConfig`（可观测）+ `AppServices`（业务入口）；基础设施仅由 `AppServices` 持有，通过 trait 暴露 |
| `main.rs` 巨型装配 | 拆分为 `bootstrap.rs`（`build_runtime()`），`main.rs` 仅调用 |
| 命令直接 new `PreTradeRiskChecker` / `ExecutionEngine` | 命令通过 `AppServices`/注入的用例获得，服务层编排 |

#### B. 后端：依赖注入收敛（DIP/ISP）

- `AppServices` 通过构造注入依赖，暴露**窄接口**（`OrderPort`, `MarketPort`, `RiskPort`, `AuthPort`…）而非全部 `pub` 字段。
- `OkxExecutor` / `OkxDataSource`：仅保留**单一实例**，通过 `Arc` 共享，消除 `main.rs` 重复构造。
- 删除对 trait-object 加 `RwLock` 的不必要包裹（除非并发写），使接口更符合 ISP。

#### C. 前端：DRY 收敛

| 现状 | 目标 |
|------|------|
| `useFormat.ts` + `useFormatting.ts` | 合并为单一 `useFormatting`（权威实现），全项目 import 统一，删除 `useFormat.ts` |
| `MetricCard.vue` + `StatsCard.vue` | 合并为 `StatCard.vue`（含 loading/trend/click 抽象），删除重复组件 |
| `services/market.ts` 混合 ws | 拆出 `services/ws.ts`（订阅管理），`market.ts` 只留行情读取 |
| `services/okx.ts` 混合下单+行情 | 拆出 `services/okxOrder.ts`（下单/撤单） |
| 各 service 重复样板 | 增加 `services/transport.ts`（统一 `invoke` 封装），消除样板 |

#### D. 前端：store 拆分（SRP/SoC）

| `stores/strategy.ts` | 拆为 `stores/strategy.ts`（CRUD/选择）+ `stores/strategyLifecycle.ts`（启停/轮询）|
| 各 store 手写 loading/error | 抽 `stores/factory.ts` 提供 `createAsyncState()` 组合函数 |

---

## 3. 重构批次安排（实现有限批边界，每批保持构建绿色）

| 批 | 范围 | 风险 | 验证 |
|----|------|------|------|
| **B1** | 前端 DRY：合并 `useFormat` → `useFormatting` | 低 | `vue-tsc` + `npm test` |
| **B2** | 后端命令层分层：提取 `submit_order` 为 `OrderProcessor` 用例，命令变薄壳 | 中 | `cargo check` + `cargo test` |
| **B3** | 前端 DRY：合并 `MetricCard`/`StatsCard` → `StatCard` | 低 | `vue-tsc` + `npm test` |
| **B4** | 前端 SoC：拆 `services/market.ts` ws、`services/okx.ts` 下单 | 低 | `vue-tsc` + `npm test` |
| **B5** | 后端装配收敛：`AppState`/`AppServices` 依赖收敛、单一 `OkxExecutor` 实例 | 中 | `cargo check` |
| **B6** | 前端 store 拆分 + transport 抽象 | 中 | `vue-tsc` + `npm test` |

> 每个批次结束：运行对应验证命令，全部通过才进入下一批。批次间互相独立，可单独回滚。

---

## 4. 不做什么（明确排除 — YAGNI / KISS）

- **不引入** 新的外部框架、消息总线、微服务拆分——当前规模（单机桌面 + 1 交易所）不需要。
- **不改业务行为**：不新增/删除/变更任何可观察功能，仅做结构与内聚重构。
- **不做** 无收益的过度抽象（如为单一实现再包一层 trait）。
- **不做** schema/迁移变更：重构不触碰数据库结构。

---

## 5. 验收标准（对应各批次）

1. `cargo check --workspace` 通过；`cargo test` 通过；无新增 clippy 告警。
2. `vue-tsc --noEmit` 通过；`npm test`（426 用例）全部通过。
3. 命令层不再直接依赖 `risk_layer`/`trading_layer`/`data_layer`/`monitor_layer`（可由依赖图验证）。
4. `useFormat.ts` 被删除，唯一的格式逻辑存在于 `useFormatting.ts`。
5. `AppServices` 不再暴露全部内部 `pub` 基础设施字段（若有暴露需有明确理由）。

---

## 6. 风险与回滚

| 风险 | 缓解 |
|------|------|
| 后端用例重构引入行为差异 | 用例保持与现存 `submit_order` 完全等价；由既有单测 + `test-plan.md` 兜底 |
| 前端合并组件改变 UI | 合并后运行全部组件测试；保留等价 prop 行为 |
| 批次间依赖 | 批次独立可回滚；每批结束时单独提交可回退点 |
