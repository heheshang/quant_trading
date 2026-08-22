use crate::error::{ServiceError, ServiceResult};
use data_layer::market_data::DataSource;
use data_layer::OkxDataSource;
use exchange_okx::types::*;
use exchange_okx::ClientInterface;
use quant_common::types::MarketData;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, instrument};

type SharedClient = Arc<RwLock<dyn ClientInterface + Send + Sync>>;

/// Executes a block with a borrowed OKX client, handling the double-RwLock guard boilerplate.
/// Returns `Err(ServiceError::OkxNotInitialized)` when the client is absent.
macro_rules! with_okx_client {
    ($self:expr, |$client:ident| $body:expr) => {{
        let guard = $self.okx_client.read().await;
        match guard.as_ref() {
            Some(client_arc) => {
                let $client = client_arc.read().await;
                $body
            }
            None => Err(ServiceError::OkxNotInitialized),
        }
    }};
}

/// OKX exchange operations service.
pub struct OkxService {
    okx_client: Arc<RwLock<Option<SharedClient>>>,
    okx_executor: Arc<RwLock<Option<Arc<trading_engine::OkxExecutor>>>>,
    okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
}

impl OkxService {
    pub fn new(
        okx_client: Arc<RwLock<Option<SharedClient>>>,
        okx_executor: Arc<RwLock<Option<Arc<trading_engine::OkxExecutor>>>>,
        okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
    ) -> Self {
        Self {
            okx_client,
            okx_executor,
            okx_data_source,
        }
    }

    pub async fn get_balance(&self, ccy: Option<&str>) -> ServiceResult<Vec<OkxBalance>> {
        with_okx_client!(self, |client| {
            client
                .get_account_balance(ccy)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_positions(&self, inst_id: Option<&str>) -> ServiceResult<Vec<OkxPosition>> {
        with_okx_client!(self, |client| {
            client
                .get_positions(inst_id)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    #[instrument(skip(self, request), fields(symbol = %request.inst_id, side = %request.side, ord_type = %request.ord_type))]
    pub async fn place_order(&self, request: OkxPlaceOrderRequest) -> ServiceResult<OkxOrder> {
        with_okx_client!(self, |client| {
            client.place_order(request).await.map_err(|e| {
                error!("Place order failed: {}", e);
                ServiceError::OkxApi(e.to_string())
            })
        })
    }

    #[instrument(skip(self), fields(inst_id = %inst_id, order_id = %ord_id))]
    pub async fn cancel_order(&self, inst_id: &str, ord_id: &str) -> ServiceResult<()> {
        with_okx_client!(self, |client| {
            client.cancel_order(inst_id, ord_id).await.map_err(|e| {
                error!("Cancel order failed: {}", e);
                ServiceError::OkxApi(e.to_string())
            })
        })
    }

    pub async fn get_candles(
        &self,
        inst_id: &str,
        bar: &str,
        limit: Option<u32>,
    ) -> ServiceResult<Vec<OkxCandle>> {
        with_okx_client!(self, |client| {
            client
                .get_candles(inst_id, bar, limit)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_instruments(&self, inst_type: &str) -> ServiceResult<serde_json::Value> {
        with_okx_client!(self, |client| {
            client
                .get_instruments(inst_type)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_ticker(&self, inst_id: &str) -> ServiceResult<OkxTicker> {
        with_okx_client!(self, |client| {
            client
                .get_ticker(inst_id)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_funding_rate(&self, inst_id: &str) -> ServiceResult<OkxFundingRate> {
        with_okx_client!(self, |client| {
            client
                .get_funding_rate(inst_id)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_mark_price(&self, inst_id: &str) -> ServiceResult<OkxMarkPrice> {
        with_okx_client!(self, |client| {
            client
                .get_mark_price(inst_id)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_index_price(&self, inst_id: &str) -> ServiceResult<OkxIndexPrice> {
        with_okx_client!(self, |client| {
            client
                .get_index_price(inst_id)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_open_interest(&self, inst_id: &str) -> ServiceResult<OkxOpenInterest> {
        with_okx_client!(self, |client| {
            client
                .get_open_interest(inst_id)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_trades(
        &self,
        inst_id: &str,
        limit: Option<u32>,
    ) -> ServiceResult<Vec<OkxTrade>> {
        with_okx_client!(self, |client| {
            client
                .get_trades(inst_id, limit)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_order_book(
        &self,
        inst_id: &str,
        sz: Option<u32>,
    ) -> ServiceResult<OkxOrderBook> {
        with_okx_client!(self, |client| {
            client
                .get_order_book(inst_id, sz)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn check_status(&self) -> ServiceResult<serde_json::Value> {
        let guard = self.okx_client.read().await;
        Ok(serde_json::json!({
            "connected": guard.is_some(),
        }))
    }

    pub async fn get_announcements(&self) -> ServiceResult<serde_json::Value> {
        with_okx_client!(self, |client| {
            let announcements = client
                .get_announcements()
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))?;
            serde_json::to_value(announcements).map_err(|e| ServiceError::Serialization {
                what: "announcements",
                source: e,
            })
        })
    }

    pub async fn execute_order(&self, order: &quant_common::types::Order) -> ServiceResult<String> {
        let guard = self.okx_executor.read().await;
        match guard.as_ref() {
            Some(exec) => exec
                .execute_order(order)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string())),
            None => Err(ServiceError::OkxExecutorNotInitialized),
        }
    }

    pub async fn get_realtime_data(&self, symbol: &str) -> ServiceResult<MarketData> {
        let ds = self.okx_data_source.read().await;
        match ds.as_ref() {
            Some(source) => source
                .get_realtime_data(symbol)
                .await
                .map_err(|e| ServiceError::DataSource(e.to_string())),
            None => Err(ServiceError::OkxDataSourceNotInitialized),
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
            None => Err(ServiceError::OkxDataSourceNotInitialized),
        }
    }
}

#[cfg(test)]
mod tests;
