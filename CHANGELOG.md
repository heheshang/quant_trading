# Changelog

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
