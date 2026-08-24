//! Market data provider abstraction.
//!
//! Decouples StrategyService (and other services) from concrete data sources.
//! Services depend on `Arc<dyn MarketDataProvider>` instead of a concrete
//! exchange data source, enabling:
//!
//! - Unit testing with mock providers
//! - Future addition of alternative data sources without service changes

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use data_layer::{MarketDataRecord, MarketDataRepository};
use quant_common::types::MarketData;
use rust_decimal::Decimal;
use std::sync::Arc;

// Re-export the trait from quant-common to avoid cyclic dependencies.
pub use quant_common::MarketDataProvider;

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
        _timeframe: &str,
    ) -> Result<Vec<MarketData>, String> {
        if let Some(err) = &self.error {
            return Err(err.clone());
        }
        Ok(self.data.clone())
    }
}

// ─── Persistence-first provider (Postgres → Binance fallback) ─────────────

/// Default candle resolution used by the live market-data source
/// (`BinanceDataSource::get_historical_data`, see
/// `crates/data-layer/src/binance_source.rs`); kept in one place so the repository
/// query and the live fallback agree on granularity.
pub const DEFAULT_TIMEFRAME: &str = "1H";

/// Choose the default candle resolution for persistence-first resolution.
///
/// Priority:
/// 1. The first configured `data_puller.candle.bars` value — this is the
///    resolution the candle puller actually writes to Postgres, so it is the
///    one most likely to have rows for the requested range.
/// 2. [`DEFAULT_TIMEFRAME`] ("1H") when the data-puller config is empty, so the
///    repository query and the live fallback agree on granularity.
pub fn resolve_default_timeframe(candle_bars: &[String]) -> String {
    candle_bars
        .first()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| DEFAULT_TIMEFRAME.to_string())
}

/// Abstraction over the persistence-backed historical market-data store.
///
/// Implemented by [`data_layer::MarketDataRepository`] in production and
/// abstracted here so the composite provider can be unit-tested without a live
/// `PgPool`.
#[async_trait]
pub trait MarketDataStore: Send + Sync {
    /// Query candles for `instrument_id` in `timeframe` within `[from, to)`.
    async fn query_historical(
        &self,
        instrument_id: &str,
        timeframe: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<MarketDataRecord>, String>;
}

#[async_trait]
impl MarketDataStore for MarketDataRepository {
    async fn query_historical(
        &self,
        instrument_id: &str,
        timeframe: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<MarketDataRecord>, String> {
        self.query_by_range(instrument_id, timeframe, from, to, None)
            .await
            .map_err(|e| e.to_string())
    }
}

/// Persistence-first market data provider.
///
/// Resolves the requested historical series from the local Postgres
/// `market_data` table at the configured default timeframe, falling back to the
/// configured live source when the repository is unconfigured, empty, or errors.
///
/// This is what the strategy scheduler uses so it can generate signals from
/// real (data-puller-written) candles without requiring exchange credentials.
pub struct RepositoryMarketDataProvider {
    repo: Option<Arc<dyn MarketDataStore>>,
    live: Option<Arc<dyn MarketDataProvider>>,
    default_timeframe: String,
}

impl RepositoryMarketDataProvider {
    /// Create a persistence-first provider.
    ///
    /// - `repo`: optional persistence store (`MarketDataRepository` in prod).
    /// - `live`: optional live fallback data source (implements
    ///   [`MarketDataProvider`]).
    /// - `default_timeframe`: candle resolution, see [`resolve_default_timeframe`].
    pub fn new(
        repo: Option<Arc<dyn MarketDataStore>>,
        live: Option<Arc<dyn MarketDataProvider>>,
        default_timeframe: String,
    ) -> Self {
        Self {
            repo,
            live,
            default_timeframe,
        }
    }
}

#[async_trait]
impl MarketDataProvider for RepositoryMarketDataProvider {
    async fn get_historical_data(
        &self,
        symbol: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        timeframe: &str,
    ) -> Result<Vec<MarketData>, String> {
        let requested = if timeframe.is_empty() {
            self.default_timeframe.as_str()
        } else {
            timeframe
        };
        // 1. Persistence-first: query Postgres `market_data` at the requested
        //    timeframe. A non-empty result wins.
        if let Some(repo) = &self.repo {
            match repo.query_historical(symbol, requested, start, end).await {
                Ok(records) if !records.is_empty() => {
                    return Ok(records.into_iter().map(record_to_market_data).collect());
                }
                Ok(_) => {
                    // No rows at the requested timeframe → fall through to the live source.
                }
                Err(e) => {
                    tracing::warn!(
                        symbol = %symbol,
                        error = %e,
                        "Repository market-data query failed; falling back to the live source"
                    );
                }
            }
        }

        // 2. Fall back to the configured live source (implements `MarketDataProvider`).
        match &self.live {
            Some(source) => source.get_historical_data(symbol, start, end, timeframe).await,
            None => Err(format!("no market data source available for {}", symbol)),
        }
    }
}

/// Map a persisted candle row to a [`MarketData`].
///
/// The `market_data` table stores OHLCV only; exchange order-book /
/// open-interest fields are unavailable, so they are left at their honest
/// defaults.
fn record_to_market_data(record: MarketDataRecord) -> MarketData {
    MarketData {
        symbol: record.instrument_id,
        timestamp: record.timestamp,
        open: record.open,
        high: record.high,
        low: record.low,
        close: record.close,
        volume: record.volume,
        turnover: Decimal::ZERO,
        open_interest: None,
        bid_prices: vec![],
        bid_volumes: vec![],
        ask_prices: vec![],
        ask_volumes: vec![],
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
            .get_historical_data("TEST", Utc::now(), Utc::now(), "1H")
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn test_mock_provider_returns_error() {
        let provider = MockMarketDataProvider::with_error("network error");
        let result = provider
            .get_historical_data("TEST", Utc::now(), Utc::now(), "1H")
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "network error");
    }

    #[tokio::test]
    async fn test_mock_provider_empty_data() {
        let provider = MockMarketDataProvider::new(vec![]);
        let result = provider
            .get_historical_data("TEST", Utc::now(), Utc::now(), "1H")
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ── RepositoryMarketDataProvider ─────────────────────────────────────

    /// Minimal persistence-store double for exercising the repo path.
    struct MockStore {
        records: Vec<MarketDataRecord>,
        error: Option<String>,
    }

    #[async_trait]
    impl MarketDataStore for MockStore {
        async fn query_historical(
            &self,
            _instrument_id: &str,
            _timeframe: &str,
            _from: DateTime<Utc>,
            _to: DateTime<Utc>,
        ) -> Result<Vec<MarketDataRecord>, String> {
            if let Some(e) = &self.error {
                return Err(e.clone());
            }
            Ok(self.records.clone())
        }
    }

    fn make_record(symbol: &str, ts: DateTime<Utc>, i: i64) -> MarketDataRecord {
        MarketDataRecord {
            id: i,
            instrument_id: symbol.to_string(),
            timeframe: "1H".to_string(),
            timestamp: ts,
            open: Decimal::new(100 + i, 2),
            high: Decimal::new(101 + i, 2),
            low: Decimal::new(99 + i, 2),
            close: Decimal::new(100 + i, 2),
            volume: Decimal::new(1000 + i, 0),
            created_at: Some(ts),
        }
    }

    #[tokio::test]
    async fn test_repo_data_returns_mapped_rows() {
        let ts = Utc::now();
        let records = vec![
            make_record("BTC-USDT", ts, 1),
            make_record("BTC-USDT", ts, 2),
        ];
        let store = Arc::new(MockStore {
            records: records.clone(),
            error: None,
        }) as Arc<dyn MarketDataStore>;
        let provider = RepositoryMarketDataProvider::new(Some(store), None, "1H".to_string());
        let result = provider
            .get_historical_data(
                "BTC-USDT",
                ts - chrono::Duration::hours(1),
                ts + chrono::Duration::hours(1),
                "1H",
            )
            .await;
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.len(), 2);
        // Field mapping from MarketDataRecord → MarketData.
        assert_eq!(data[0].symbol, "BTC-USDT");
        assert_eq!(data[0].timestamp, ts);
        // record i=1 → open=1.01, high=1.02, low=1.00, close=1.01, volume=1001
        assert_eq!(data[0].open, Decimal::new(101, 2));
        assert_eq!(data[0].high, Decimal::new(102, 2));
        assert_eq!(data[0].low, Decimal::new(100, 2));
        assert_eq!(data[0].close, Decimal::new(101, 2));
        assert_eq!(data[0].volume, Decimal::new(1001, 0));
        // Unavailable fields are left at their honest defaults.
        assert_eq!(data[0].turnover, Decimal::ZERO);
        assert_eq!(data[0].open_interest, None);
        assert!(data[0].bid_prices.is_empty());
        assert!(data[0].ask_prices.is_empty());
    }

    #[tokio::test]
    async fn test_empty_repo_falls_back_to_binance() {
        let ts = Utc::now();
        let binance_data = vec![MarketData {
            symbol: "BTC-USDT".to_string(),
            timestamp: ts,
            open: Decimal::new(50, 2),
            high: Decimal::new(51, 2),
            low: Decimal::new(49, 2),
            close: Decimal::new(50, 2),
            volume: Decimal::new(10, 0),
            turnover: Decimal::new(100, 0),
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        }];
        let store = Arc::new(MockStore {
            records: vec![],
            error: None,
        }) as Arc<dyn MarketDataStore>;
        let binance = Arc::new(MockMarketDataProvider::new(binance_data.clone()))
            as Arc<dyn MarketDataProvider>;
        let provider =
            RepositoryMarketDataProvider::new(Some(store), Some(binance), "1H".to_string());
        let result = provider.get_historical_data("BTC-USDT", ts, ts, "1H").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_no_sources_returns_clear_error() {
        let ts = Utc::now();
        let provider = RepositoryMarketDataProvider::new(None, None, "1H".to_string());
        let err = provider
            .get_historical_data("BTC-USDT", ts, ts, "1H")
            .await
            .unwrap_err();
        assert!(err.contains("no market data source available for BTC-USDT"));
    }

    #[tokio::test]
    async fn test_empty_repo_and_no_binance_returns_clear_error() {
        let ts = Utc::now();
        let store = Arc::new(MockStore {
            records: vec![],
            error: None,
        }) as Arc<dyn MarketDataStore>;
        let provider = RepositoryMarketDataProvider::new(Some(store), None, "1H".to_string());
        let err = provider
            .get_historical_data("BTC-USDT", ts, ts, "1H")
            .await
            .unwrap_err();
        assert!(err.contains("no market data source available for BTC-USDT"));
    }

    #[test]
    fn test_resolve_default_timeframe() {
        assert_eq!(resolve_default_timeframe(&["1m".into(), "5m".into()]), "1m");
        assert_eq!(resolve_default_timeframe(&["1H".into()]), "1H");
        assert_eq!(resolve_default_timeframe(&[]), DEFAULT_TIMEFRAME);
        assert_eq!(resolve_default_timeframe(&["".into()]), DEFAULT_TIMEFRAME);
    }
}
