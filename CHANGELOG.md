# Changelog

## [0.2.1] - 2026-08-22

### Changed

- **架构重构（遵循软件设计 7 原则）**：在不改变外部行为的前提下，系统性收敛设计债，净减少约 494 行。
  - **命令层薄壳**：新增 `OrderProcessor` 用例承载完整下单编排（行情→风控→提交→持久化→事件→异步执行）；`submit_order` 由 ~110 行「上帝函数」变为薄适配器（SRP / 分层 / DIP）。
  - **装配收敛**：引入 `SharedInfra` 打包注入，`AppServices` 构造参数由 10 → 2；消除 `OkxExecutor` 重复实例化，命令层与服务层共享同一 `Arc<OkxExecutor>`（DIP / DRY）。
  - **分层加固**：`get_market_data` / `get_okx_realtime_data` / `get_okx_historical_data` 改经 `market_service`，命令层不再直接依赖 `data_layer`。
  - **前端 DRY**：合并 `useFormat` → 权威 `useFormatting`；删除死代码 `MetricCard.vue`。
  - **前端 SoC**：拆分 `services/market` + `ws`、`services/okx` + `okxOrder`。
  - **前端 SRP**：`stores/strategy` 拆分为 `strategy`（数据/CRUD/类型/轮询）+ `strategyLifecycle`（生命周期动作，组合 base store）。
  - **前端 DIP**：新增 `transport.ts`，服务层不再直接 `import @tauri-apps/api`，框架依赖收敛到单点。

### Tests

- 验证：`cargo test --workspace` 559 passed / 0 failed / 17 ignored；`cargo clippy --all-targets` 0 warning；`vue-tsc` 通过；`npm test` 34 文件 / 426 passed。
- 外部行为零变化（测试基线一致，仅新增 `strategyLifecycle.store.test.ts`）。

### Notes

- `pre_trade_check` / `get_risk_metrics` 保留 `risk_layer` 直连，属**有意风控领域边界**（需返回具体失败原因并触发告警）；经当前 `risk_service.pre_trade_check` 会丢失失败明细，进阶需先增强 service 签名。

## [0.2.0] - 2026-08-22

### Added

- **Market data database migrations**: Added `market_data` table with RANGE partitioning for time-series data, plus JSON fields on existing tables (`orders.extra`, `positions.extra`, `strategies.extra`, `alerts.extra`, `risk_metrics.extra`) for extensible metadata storage.
- **Market data repository**: New `MarketDataRepository` module with methods for querying and storing market data, including kline and ticker queries with time-range filters.
- **Technical indicators**: Added RSI (Relative Strength Index), EMA (Exponential Moving Average), MACD (Moving Average Convergence Divergence), and Bollinger Bands indicators to the strategy layer. All indicators include comprehensive unit tests.
- **Indicator error type**: Unified `IndicatorError` enum with typed variants for calculation, configuration, and data errors, integrating with the existing `quant_common::Error` system.
- **Docker deployment**: Added `Dockerfile`, `compose.yaml`, `.dockerignore`, container entrypoint and `scripts/docker-test.sh` for one-command PostgreSQL/Redis/app orchestration and smoke testing.
- **Code flow documentation**: Added `docs/CODE_FLOW.md` (architecture, startup and six business flows) and `docs/CODE_AUDIT.md` (per-feature audit).
- **Startup resilience**: Database connection pool is now lazily initialized with background retry, so the app boots without waiting for PostgreSQL.

### Changed

- **Migration integration tests**: Extended migration test coverage to verify the new `market_data` table and JSON field migrations, including rollback verification.
- **Data layer exports**: Updated `lib.rs` to export new migration IDs and the `MarketDataRepository` module.
- **Schema generation**: Auto-generated Tauri ACL and desktop/macOS schema files to reflect new state including `redis_cache` and `pg_client` fields.
- **sqlx 0.8 upgrade**: Root workspace `sqlx` bumped 0.7 → 0.8 with `tls-rustls-ring` and explicit `derive`/`macros` features.
- **OKX WebSocket fixes**: Single-channel unsubscribe, runtime dynamic subscription, graceful shutdown, fair heartbeat scheduling and subscription dedup.
- **Tauri version alignment**: `@tauri-apps/api`/`cli` aligned to the 2.11 minor with Rust `tauri 2.11.5`.
- **PostgreSQL connect timeout**: Added `DatabaseConfig.connect_timeout_seconds` (default 3s, `DATABASE_CONNECT_TIMEOUT_SECONDS` override), shrinking no-DB startup from ~30s to ~3s.

### Infrastructure

- **Migration SQL scripts**: Placed migration SQL files in `crates/data-layer/migrations/` for standalone execution.
- **Fix**: `20240101000013_add_strategy_status_and_fields.sql` no longer fails on fresh databases (kept `strategy_id` primary key, added unique constraint on `id`).
