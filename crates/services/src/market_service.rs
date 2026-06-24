use crate::error::{ServiceError, ServiceResult};
use data_layer::market_data::DataSource;
use data_layer::OkxDataSource;
use quant_common::types::MarketData;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Market data service — retrieves real-time and historical data.
pub struct MarketService {
    okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
}

impl MarketService {
    pub fn new(okx_data_source: Arc<RwLock<Option<OkxDataSource>>>) -> Self {
        Self { okx_data_source }
    }

    pub async fn get_realtime_data(&self, symbol: &str) -> ServiceResult<MarketData> {
        let ds = self.okx_data_source.read().await;
        match ds.as_ref() {
            Some(source) => source
                .get_realtime_data(symbol)
                .await
                .map_err(|e| ServiceError::DataSource(e.to_string())),
            None => Err(ServiceError::Other(
                "OKX data source not available (check API configuration)".into(),
            )),
        }
    }

    pub async fn get_historical_data(
        &self,
        symbol: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> ServiceResult<Vec<MarketData>> {
        let ds = self.okx_data_source.read().await;
        match ds.as_ref() {
            Some(source) => source
                .get_historical_data(symbol, start, end)
                .await
                .map_err(|e| ServiceError::DataSource(e.to_string())),
            None => Err(ServiceError::Other("OKX data source not available".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_realtime_data_no_datasource() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)));
        let result = svc.get_realtime_data("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ServiceError::Other(_)));
    }

    #[tokio::test]
    async fn test_get_historical_data_no_datasource() {
        let svc = MarketService::new(Arc::new(RwLock::new(None)));
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
}
