//! # Repository Layer (part of data-layer)
//!
//! Database access patterns. Contains Repository trait definitions and
//! PostgreSQL implementations for per-entity persistence (strategies,
//! backtests, alerts, audit logs, API keys).
//!
//! ## Design
//! - Each entity has a Repository trait (defined here) + PG implementation
//! - Row types map 1:1 to database tables
//! - No business logic — pure data access
//!
//! Connection pool and migration management are owned by
//! [`crate::postgres::PostgresClient`].

pub mod alerts;
pub mod api_key;
pub mod audit;
pub mod backtest;
pub mod error;
pub mod strategy_repository;

pub use alerts::{AlertRepository, PgAlertRepository};
pub use api_key::{ApiKeyRecord, ApiKeyRepository, NewApiKey, PgApiKeyRepository};
pub use audit::{AuditFilter, AuditLogRecord, AuditRepository, NewAuditLog, PgAuditRepository};
pub use backtest::{
    BacktestRepository, BacktestResultSummaryRow, BacktestResultsPage, PgBacktestRepository,
};
pub use error::RepoError;
pub use strategy_repository::{
    PgStrategyRepository, StrategyRepository, StrategyStats, StrategySummaryRow,
};
