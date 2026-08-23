use crate::error::{ServiceError, ServiceResult};
use data_layer::market_data::DataSource;
use data_layer::{
    AccountSnapshotRecord, FundingRateRecord, MarkPriceRecord, MarketDataRepository,
    PositionSnapshotRecord, TickerSnapshotRecord,
};
use quant_common::types::MarketData;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, instrument};

/// Market data service — retrieves real-time and historical data, and reads
/// persisted snapshots/funding/mark-price series from the repository.
pub struct MarketService {
    data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>,
    market_data: Option<Arc<MarketDataRepository>>,
}

impl MarketService {
    pub fn new(
        data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>,
        market_data: Option<Arc<MarketDataRepository>>,
    ) -> Self {
        Self {
            data_source,
            market_data,
        }
    }

    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_realtime_data(&self, symbol: &str) -> ServiceResult<MarketData> {
        let ds = self.data_source.read().await;
        match ds.as_ref() {
            Some(source) => source.get_realtime_data(symbol).await.map_err(|e| {
                error!(symbol = %symbol, "Failed to get realtime data: {}", e);
                ServiceError::DataSource(e.to_string())
            }),
            None => {
                error!("market data source not configured for realtime data");
                Err(ServiceError::Other(
                    "market data source not configured (check exchange API configuration)".into(),
                ))
            }
        }
    }

    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_historical_data(
        &self,
        symbol: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> ServiceResult<Vec<MarketData>> {
        let ds = self.data_source.read().await;
        match ds.as_ref() {
            Some(source) => source
                .get_historical_data(symbol, start, end)
                .await
                .map_err(|e| {
                    error!(symbol = %symbol, "Failed to get historical data: {}", e);
                    ServiceError::DataSource(e.to_string())
                }),
            None => {
                error!("market data source not configured for historical data");
                Err(ServiceError::Other(
                    "market data source not configured".into(),
                ))
            }
        }
    }

    /// Read persisted ticker snapshots for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_ticker_snapshots(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<TickerSnapshotRecord>> {
        let repo = self.repo_or_err("ticker snapshots not available (no database)")?;
        repo.query_ticker_snapshots(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query ticker snapshots: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted account snapshots for a currency.
    #[instrument(skip(self), fields(ccy = %ccy))]
    pub async fn get_account_snapshots(
        &self,
        ccy: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<AccountSnapshotRecord>> {
        let repo = self.repo_or_err("account snapshots not available (no database)")?;
        repo.query_account_snapshots(ccy, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query account snapshots: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted position snapshots for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_position_snapshots(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<PositionSnapshotRecord>> {
        let repo = self.repo_or_err("position snapshots not available (no database)")?;
        repo.query_position_snapshots(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query position snapshots: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted funding rates for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_funding_rates(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<FundingRateRecord>> {
        let repo = self.repo_or_err("funding rates not available (no database)")?;
        repo.query_funding_rates(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query funding rates: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    /// Read persisted mark prices for an instrument.
    #[instrument(skip(self), fields(inst_id = %inst_id))]
    pub async fn get_mark_prices(
        &self,
        inst_id: &str,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
    ) -> ServiceResult<Vec<MarkPriceRecord>> {
        let repo = self.repo_or_err("mark prices not available (no database)")?;
        repo.query_mark_prices(inst_id, from, to, limit)
            .await
            .map_err(|e| {
                error!("Failed to query mark prices: {}", e);
                ServiceError::Other(e.to_string())
            })
    }

    fn repo_or_err(&self, msg: &str) -> ServiceResult<Arc<MarketDataRepository>> {
        self.market_data.clone().ok_or_else(|| {
            error!("{}", msg);
            ServiceError::Other(msg.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_realtime_data_no_datasource() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)), None);
        let result = svc.get_realtime_data("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Other(_)));
    }

    #[tokio::test]
    async fn test_get_historical_data_no_datasource() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)), None);
        let result = svc
            .get_historical_data(
                "BTC-USDT",
                chrono::Utc::now() - chrono::Duration::days(7),
                chrono::Utc::now(),
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Other(_)));
    }

    #[tokio::test]
    async fn test_get_funding_rates_no_repo() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)), None);
        let result = svc
            .get_funding_rates("BTC-USDT", None, None, Some(10))
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Other(_)));
    }

    #[tokio::test]
    async fn test_get_mark_prices_no_repo() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)), None);
        let result = svc.get_mark_prices("BTC-USDT", None, None, Some(10)).await;
        assert!(result.is_err());
    }
}
