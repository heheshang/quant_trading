//! Market data provider abstraction.
//!
//! Decouples strategy-layer and services from concrete data sources.
//! Lives in `quant-common` to avoid cyclic dependencies between crates.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::types::MarketData;

/// Abstraction over historical market data access.
///
/// The minimal interface needed by backtesting and strategy execution.
#[async_trait]
pub trait MarketDataProvider: Send + Sync {
    /// Fetch historical market data for the given symbol and date range.
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>, String>;
}
