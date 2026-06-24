//! # Quant Trading — Clients Layer
//!
//! External service call wrappers: cache (Redis) and future exchange API clients.
//!
//! ## Design
//! - Every external service gets a trait for testability
//! - Implementations are injectable
//! - No business logic — pure I/O with error handling

pub mod error;
pub mod redis_cache;

pub use error::ClientError;
pub use redis_cache::RedisCache;
