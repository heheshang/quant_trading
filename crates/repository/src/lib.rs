//! # Quant Trading — Repository Layer
//!
//! Database access patterns. Contains connection management and
//! Repository trait implementations for PostgreSQL.
//!
//! ## Design
//! - Each entity has a Repository trait (defined here) + PG implementation
//! - Row types map 1:1 to database tables
//! - No business logic — pure data access

pub mod backtest;
pub mod error;
pub mod market_data;
pub mod postgres;
pub mod strategy_repository;

pub use backtest::{BacktestRepository, BacktestResultSummaryRow, PgBacktestRepository};
pub use error::RepoError;
pub use market_data::MarketDataRepository;
pub use postgres::PostgresClient;
pub use strategy_repository::{
    PgStrategyRepository, StrategyRepository, StrategyStats, StrategySummaryRow,
};
