use crate::error::{ServiceError, ServiceResult};
use data_layer::market_data::DataSource;
use data_layer::OkxDataSource;
use exchange_okx::types::*;
use exchange_okx::Client as OkxClient;
use quant_common::types::MarketData;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, instrument};

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
    okx_client: Arc<RwLock<Option<Arc<RwLock<OkxClient>>>>>,
    okx_executor: Arc<RwLock<Option<Arc<trading_engine::OkxExecutor>>>>,
    okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
}

impl OkxService {
    pub fn new(
        okx_client: Arc<RwLock<Option<Arc<RwLock<OkxClient>>>>>,
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
            client
                .place_order(request)
                .await
                .map_err(|e| {
                    error!("Place order failed: {}", e);
                    ServiceError::OkxApi(e.to_string())
                })
        })
    }

    #[instrument(skip(self), fields(inst_id = %inst_id, order_id = %ord_id))]
    pub async fn cancel_order(&self, inst_id: &str, ord_id: &str) -> ServiceResult<()> {
        with_okx_client!(self, |client| {
            client
                .cancel_order(inst_id, ord_id)
                .await
                .map_err(|e| {
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
mod tests {
    use super::*;
    use quant_common::types::{Order, OrderSide, OrderStatus, OrderType};

    fn make_service() -> OkxService {
        OkxService::new(
            Arc::new(RwLock::new(None)),
            Arc::new(RwLock::new(None)),
            Arc::new(RwLock::new(None)),
        )
    }

    #[tokio::test]
    async fn test_get_balance_not_initialized() {
        let svc = make_service();
        let result = svc.get_balance(None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_positions_not_initialized() {
        let svc = make_service();
        let result = svc.get_positions(None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_place_order_not_initialized() {
        let svc = make_service();
        let request = OkxPlaceOrderRequest {
            inst_id: "BTC-USDT".into(),
            td_mode: "cash".into(),
            side: "buy".into(),
            ord_type: "limit".into(),
            sz: "1".into(),
            px: Some("50000".into()),
            cl_ord_id: None,
            tag: None,
            pos_side: None,
            ccy: None,
            px_usd: None,
            px_vol: None,
            reduce_only: None,
            tgt_ccy: None,
        };
        let result = svc.place_order(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_cancel_order_not_initialized() {
        let svc = make_service();
        let result = svc.cancel_order("BTC-USDT", "ord123").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_candles_not_initialized() {
        let svc = make_service();
        let result = svc.get_candles("BTC-USDT", "1m", Some(10)).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_instruments_not_initialized() {
        let svc = make_service();
        let result = svc.get_instruments("SPOT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_check_status_not_connected() {
        let svc = make_service();
        let result = svc.check_status().await;
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["connected"], false);
    }

    #[tokio::test]
    async fn test_get_announcements_not_initialized() {
        let svc = make_service();
        let result = svc.get_announcements().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_execute_order_not_initialized() {
        let svc = make_service();
        let order = Order {
            order_id: uuid::Uuid::new_v4(),
            strategy_id: "strat_1".into(),
            symbol: "BTC-USDT".into(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: None,
            quantity: rust_decimal::Decimal::new(1, 0),
            filled_quantity: rust_decimal::Decimal::ZERO,
            status: OrderStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            commission: rust_decimal::Decimal::ZERO,
            slippage: rust_decimal::Decimal::ZERO,
        };
        let result = svc.execute_order(&order).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxExecutorNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_realtime_data_not_initialized() {
        let svc = make_service();
        let result = svc.get_realtime_data("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxDataSourceNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_historical_data_not_initialized() {
        let svc = make_service();
        let result = svc
            .get_historical_data(
                "BTC-USDT",
                chrono::Utc::now() - chrono::Duration::days(7),
                chrono::Utc::now(),
            )
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxDataSourceNotInitialized
        ));
    }
}
