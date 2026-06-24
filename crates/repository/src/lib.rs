//! # Quant Trading — Repository Layer
//!
//! Database access patterns. Contains connection management and
//! Repository trait implementations for PostgreSQL.
//!
//! ## Design
//! - Each entity has a Repository trait (defined here) + PG implementation
//! - Row types map 1:1 to database tables
//! - No business logic — pure data access

pub mod error;
pub mod market_data;
pub mod postgres;

pub use error::RepoError;
pub use market_data::MarketDataRepository;
pub use postgres::PostgresClient;
