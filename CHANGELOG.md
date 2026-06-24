# Changelog

## [0.2.0] - 2026-06-24

### Added

- **Market data database migrations**: Added `market_data` table with RANGE partitioning for time-series data, plus JSON fields on existing tables (`orders.extra`, `positions.extra`, `strategies.extra`, `alerts.extra`, `risk_metrics.extra`) for extensible metadata storage.
- **Market data repository**: New `MarketDataRepository` module with methods for querying and storing market data, including kline and ticker queries with time-range filters.
- **Technical indicators**: Added RSI (Relative Strength Index), EMA (Exponential Moving Average), MACD (Moving Average Convergence Divergence), and Bollinger Bands indicators to the strategy layer. All indicators include comprehensive unit tests.
- **Indicator error type**: Unified `IndicatorError` enum with typed variants for calculation, configuration, and data errors, integrating with the existing `quant_common::Error` system.

### Changed

- **Migration integration tests**: Extended migration test coverage to verify the new `market_data` table and JSON field migrations, including rollback verification.
- **Data layer exports**: Updated `lib.rs` to export new migration IDs and the `MarketDataRepository` module.
- **Schema generation**: Auto-generated Tauri ACL and desktop/macOS schema files to reflect new state including `redis_cache` and `pg_client` fields.

### Infrastructure

- **Migration SQL scripts**: Placed migration SQL files in `crates/data-layer/migrations/` for standalone execution.
