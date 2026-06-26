//! Market data provider abstraction.
//!
//! Decouples StrategyService (and other services) from concrete data sources
//! such as `OkxDataSource`. Services depend on `Arc<dyn MarketDataProvider>`
//! instead of `Arc<RwLock<Option<OkxDataSource>>>`, enabling:
//!
//! - Unit testing with mock providers
//! - Future addition of alternative data sources without service changes

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use data_layer::OkxDataSource;
use quant_common::types::MarketData;
use std::sync::Arc;
use tokio::sync::RwLock;

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

// ─── OkxDataSource implementation ───────────────────────────────────────

#[async_trait]
impl MarketDataProvider for OkxDataSource {
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>, String> {
        use data_layer::market_data::DataSource;
        <Self as DataSource>::get_historical_data(self, symbol, start, end)
            .await
            .map_err(|e| e.to_string())
    }
}

// ─── LockingProvider: bridges Arc<RwLock<Option<OkxDataSource>>> ───────

/// Adapter that wraps `Arc<RwLock<Option<OkxDataSource>>>` to implement
/// `MarketDataProvider`. Returns an error when the lock is not initialized.
///
/// This exists for backward compatibility with `AppServices` which holds
/// the data source behind a shared lock.
pub struct LockingProvider {
    inner: Arc<RwLock<Option<OkxDataSource>>>,
}

impl LockingProvider {
    pub fn new(inner: Arc<RwLock<Option<OkxDataSource>>>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl MarketDataProvider for LockingProvider {
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>, String> {
        let guard = self.inner.read().await;
        match guard.as_ref() {
            Some(source) => source.get_historical_data(symbol, start, end).await,
            None => Err("Data source not initialized".to_string()),
        }
    }
}

// ─── Mock implementation for testing ────────────────────────────────────

/// Mock provider that returns empty data or a predefined error.
pub struct MockMarketDataProvider {
    pub data: Vec<MarketData>,
    pub error: Option<String>,
}

impl MockMarketDataProvider {
    pub fn new(data: Vec<MarketData>) -> Self {
        Self { data, error: None }
    }

    pub fn with_error(msg: impl Into<String>) -> Self {
        Self {
            data: vec![],
            error: Some(msg.into()),
        }
    }
}

#[async_trait]
impl MarketDataProvider for MockMarketDataProvider {
    async fn get_historical_data(
        &self,
        _symbol: &str,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>, String> {
        if let Some(err) = &self.error {
            return Err(err.clone());
        }
        Ok(self.data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn make_mock_data(count: usize) -> Vec<MarketData> {
        (0..count)
            .map(|i| MarketData {
                symbol: "TEST".to_string(),
                timestamp: Utc::now(),
                open: Decimal::new(100 + i as i64, 2),
                high: Decimal::new(101 + i as i64, 2),
                low: Decimal::new(99 + i as i64, 2),
                close: Decimal::new(100 + i as i64, 2),
                volume: Decimal::new(1000, 0),
                turnover: Decimal::new(100000, 2),
                open_interest: None,
                bid_prices: vec![],
                bid_volumes: vec![],
                ask_prices: vec![],
                ask_volumes: vec![],
            })
            .collect()
    }

    #[tokio::test]
    async fn test_mock_provider_returns_data() {
        let data = make_mock_data(5);
        let provider = MockMarketDataProvider::new(data.clone());
        let result = provider
            .get_historical_data("TEST", Utc::now(), Utc::now())
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn test_mock_provider_returns_error() {
        let provider = MockMarketDataProvider::with_error("network error");
        let result = provider
            .get_historical_data("TEST", Utc::now(), Utc::now())
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "network error");
    }

    #[tokio::test]
    async fn test_mock_provider_empty_data() {
        let provider = MockMarketDataProvider::new(vec![]);
        let result = provider
            .get_historical_data("TEST", Utc::now(), Utc::now())
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
