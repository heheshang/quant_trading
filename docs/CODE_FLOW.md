# 量化交易系统代码流程

## 1. 系统分层

```text
Vue3 前端 (src/)
  ├── views           页面与交互编排
  ├── stores          Pinia 状态管理
  ├── services        Tauri invoke 封装与类型契约
  ├── composables     可复用组合式逻辑
  └── components      业务组件
        │ invoke / listen
        ▼
Tauri 后端 (src-tauri/)
  ├── main.rs         启动、依赖装配、命令注册
  ├── commands.rs     业务命令（配置/账户/策略/风控/OKX）
  └── ws_commands.rs  实时行情 WebSocket 命令
        │
        ▼
服务层 (crates/services)
  AppServices 装配 Auth/Account/Market/Strategy/Risk/Okx/Config
        │
        ├───────────────────────────────────┐
        ▼                                   ▼
领域/策略/交易/风控                     基础设施
strategy-layer                         data-layer / repository / clients
trading-layer                          PostgreSQL / Redis / OKX REST / WS
risk-layer
monitor-layer
```

## 2. 启动流程

1. `src-tauri/src/main.rs` 加载 `.env` 与 `AppConfig::default()`。
2. `monitor_layer::logging::init_logging` 初始化结构化日志。
3. 初始化 `AlertManager` 与 `LogBuffer`。
4. 尝试初始化 OKX `Client`；成功时创建 `OkxExecutor`、`OkxDataSource`，失败则降级为 `None`。
5. 尝试初始化 `data_layer::PostgresClient` 并运行 migration；失败则无数据库降级运行。
6. 初始化 Redis、`DataPuller`、`OrderManager`。
7. `AppServices` 装配业务服务，注入共享配置、数据库、OKX 与策略调度器。
8. 注册 Tauri 命令并启动窗口。

## 3. 六大业务链路

### 3.1 行情链路

```text
OKX REST / WebSocket
  → exchange-okx Client / OkxWebSocket
  → OkxDataSource / data-puller DataPuller
  → data-layer MarketDataRepository (PostgreSQL / Redis)
  → MarketService
  → Tauri 命令 get_market_data / ws:ticker
  → 前端 stores/marketData + useMarketData
```

### 3.2 策略链路

```text
前端 Strategy.vue
  → services/strategy.ts
  → get/save/delete/deploy/start/stop/pause/resume/archive
  → StrategyService
  → strategy-layer registry + strategy.rs + scheduler
  → repository PgStrategyRepository
```

### 3.3 回测链路

```text
前端 Backtest.vue
  → services/backtest.ts
  → StrategyService.run_backtest
  → BacktestEngine + 参数 schema 校验
  → PgBacktestRepository 持久化
  → 前端图表与历史列表
```

### 3.4 交易链路

```text
前端 Trading.vue
  → services/order.ts / services/okx.ts
  → submit_order / place_okx_order / cancel_okx_order / execute_okx_order
  → OrderManager + ExecutionEngine + OkxExecutor
  → risk-layer PreTradeRiskChecker（可选）
  → OKX REST + AccountService.persist_order
  → order:submitted 事件回流前端
```

### 3.5 风控链路

```text
前端 Risk.vue
  → services/risk.ts
  → get_risk_metrics / get_risk_config / update_risk_config / pre_trade_check
  → RiskService
  → risk-layer pre_trade / real_time / post_trade / var
```

### 3.6 监控链路

```text
前端 Monitor.vue
  → services/monitor.ts
  → get_metrics / get_alerts / acknowledge_alert / get_logs
  → monitor-layer MetricsCollector / AlertManager / LogBuffer
  → ws:alerts / ws:logs / ws:ticker
```

## 4. 前后端命名约定

- Tauri 命令统一 `snake_case`。
- Rust 序列化字段使用 `camelCase` 的 OKX 类型以 `camelCase` 透传，例如
  `OkxBalance.availEq`、`OkxOrder.ordId`、`OkxInstrument.instId`。
- 系统业务类型（账户、订单、策略、回测）保持 `snake_case`。

## 5. 已知关键优化点

- `main.rs` 曾创建两套 PostgreSQL 连接池；本次改为复用 `data_layer::PostgresClient`
  的 `PgPool` 构造 repository 客户端。
- `AlertManager` 曾直接使用 `reqwest::Client::new()`，在 macOS 无头环境会读取系统代理
  并触发 panic；本次改为禁用系统代理并设置超时。
- Rust 单元测试不应依赖外部 PostgreSQL；连接类测试改为 `#[ignore]`，真实环境可显式运行。
- 前端缺少统一 `test` script，且 Vitest setup 未提供稳定的 `localStorage` polyfill。
- npm `@tauri-apps/api` / `@tauri-apps/cli` 已与 Rust `tauri 2.11.x` 对齐，避免
  `tauri dev` 启动时版本不匹配。
- PostgreSQL 连接池默认 30 秒超时改为可配置 `connect_timeout_seconds`，默认 3 秒；
  连接池改为懒加载，数据库不可用时应用先启动，再在后台重试连接和迁移。
