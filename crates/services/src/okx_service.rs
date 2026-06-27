use crate::error::{ServiceError, ServiceResult};
use data_layer::market_data::DataSource;
use data_layer::OkxDataSource;
use exchange_okx::types::*;
use exchange_okx::ClientInterface;
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
    okx_client: Arc<RwLock<Option<Arc<RwLock<dyn ClientInterface + Send + Sync>>>>>,
    okx_executor: Arc<RwLock<Option<Arc<trading_engine::OkxExecutor>>>>,
    okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
}

impl OkxService {
    pub fn new(
        okx_client: Arc<RwLock<Option<Arc<RwLock<dyn ClientInterface + Send + Sync>>>>>,
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

    pub async fn get_trades(&self, inst_id: &str, limit: Option<u32>) -> ServiceResult<Vec<OkxTrade>> {
        with_okx_client!(self, |client| {
            client
                .get_trades(inst_id, limit)
                .await
                .map_err(|e| ServiceError::OkxApi(e.to_string()))
        })
    }

    pub async fn get_order_book(&self, inst_id: &str, sz: Option<u32>) -> ServiceResult<OkxOrderBook> {
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
mod tests {
    use super::*;
    use exchange_okx::MockOkxClient;
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
            order_id: 0,
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

    // ── Helper ──────────────────────────────────────────────────────────

    fn make_service_with_mock(mock: MockOkxClient) -> OkxService {
        let dyn_client: Arc<RwLock<dyn ClientInterface + Send + Sync>> =
            Arc::new(RwLock::new(mock));
        OkxService::new(
            Arc::new(RwLock::new(Some(dyn_client))),
            Arc::new(RwLock::new(None)),
            Arc::new(RwLock::new(None)),
        )
    }

    // ── Missing not_initialized tests ───────────────────────────────────

    #[tokio::test]
    async fn test_get_ticker_not_initialized() {
        let svc = make_service();
        let result = svc.get_ticker("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_funding_rate_not_initialized() {
        let svc = make_service();
        let result = svc.get_funding_rate("BTC-USDT-SWAP").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_mark_price_not_initialized() {
        let svc = make_service();
        let result = svc.get_mark_price("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_index_price_not_initialized() {
        let svc = make_service();
        let result = svc.get_index_price("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_open_interest_not_initialized() {
        let svc = make_service();
        let result = svc.get_open_interest("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_trades_not_initialized() {
        let svc = make_service();
        let result = svc.get_trades("BTC-USDT", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    #[tokio::test]
    async fn test_get_order_book_not_initialized() {
        let svc = make_service();
        let result = svc.get_order_book("BTC-USDT", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxNotInitialized
        ));
    }

    // ── Check status ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_check_status_connected() {
        let mock = MockOkxClient::new();
        let svc = make_service_with_mock(mock);
        let result = svc.check_status().await;
        assert!(result.is_ok());
        let json = result.unwrap();
        assert_eq!(json["connected"], true);
    }

    // ── With mock tests (happy path) ────────────────────────────────────

    #[tokio::test]
    async fn test_get_balance_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_account_balance()
            .returning(|_| Box::pin(async {
                Ok(vec![
                    OkxBalance {
                        ccy: "BTC".into(),
                        eq: "1.5".into(),
                        cash_bal: "1.0".into(),
                        avail_eq: "1.5".into(),
                        frozen_bal: "0".into(),
                    },
                    OkxBalance {
                        ccy: "ETH".into(),
                        eq: "10.0".into(),
                        cash_bal: "10.0".into(),
                        avail_eq: "10.0".into(),
                        frozen_bal: "0".into(),
                    },
                ])
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_balance(Some("BTC")).await;
        assert!(result.is_ok());
        let balances = result.unwrap();
        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].ccy, "BTC");
        assert_eq!(balances[1].ccy, "ETH");
    }

    #[tokio::test]
    async fn test_get_positions_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_positions()
            .returning(|_| Box::pin(async {
                Ok(vec![
                    OkxPosition {
                        inst_id: "BTC-USDT".into(),
                        pos: "1".into(),
                        avail_pos: "1".into(),
                        avg_px: "45000.0".into(),
                        upl: "100.0".into(),
                        upl_ratio: "0.02".into(),
                    },
                    OkxPosition {
                        inst_id: "ETH-USDT".into(),
                        pos: "-5".into(),
                        avail_pos: "-5".into(),
                        avg_px: "3200.0".into(),
                        upl: "-50.0".into(),
                        upl_ratio: "-0.01".into(),
                    },
                ])
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_positions(None).await;
        assert!(result.is_ok());
        let positions = result.unwrap();
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].inst_id, "BTC-USDT");
        assert_eq!(positions[1].inst_id, "ETH-USDT");
    }

    #[tokio::test]
    async fn test_place_order_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_place_order()
            .returning(|_: OkxPlaceOrderRequest| Box::pin(async {
                Ok(OkxOrder {
                    ord_id: "mock-ord-123".into(),
                    cl_ord_id: "cl-mock".into(),
                    inst_id: "BTC-USDT".into(),
                    side: "buy".into(),
                    ord_type: "market".into(),
                    px: "0".into(),
                    sz: "1".into(),
                    state: "live".into(),
                    avg_px: "0".into(),
                    acc_fill_sz: "0".into(),
                    u_time: "1597026383085".into(),
                })
            }));
        let svc = make_service_with_mock(mock);
        let request = OkxPlaceOrderRequest {
            inst_id: "BTC-USDT".into(),
            td_mode: "cash".into(),
            side: "buy".into(),
            ord_type: "market".into(),
            sz: "1".into(),
            px: None,
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
        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.ord_id, "mock-ord-123");
        assert_eq!(order.state, "live");
    }

    #[tokio::test]
    async fn test_cancel_order_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_cancel_order()
            .returning(|_: &str, _: &str| Box::pin(async { Ok(()) }));
        let svc = make_service_with_mock(mock);
        let result = svc.cancel_order("BTC-USDT", "ord123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_candles_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_candles()
            .returning(|_: &str, _: &str, _: Option<u32>| Box::pin(async {
                Ok(vec![
                    OkxCandle {
                        ts: "1597026383000".into(),
                        open: "45000".into(),
                        high: "45500".into(),
                        low: "44900".into(),
                        close: "45200".into(),
                        vol: "100.0".into(),
                        vol_ccy: "4500000".into(),
                    },
                    OkxCandle {
                        ts: "1597026384000".into(),
                        open: "45200".into(),
                        high: "45600".into(),
                        low: "45100".into(),
                        close: "45400".into(),
                        vol: "150.0".into(),
                        vol_ccy: "6780000".into(),
                    },
                ])
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_candles("BTC-USDT", "1m", Some(2)).await;
        assert!(result.is_ok());
        let candles = result.unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].ts, "1597026383000");
    }

    #[tokio::test]
    async fn test_get_instruments_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_instruments()
            .returning(|_: &str| Box::pin(async {
                Ok(serde_json::json!([{
                    "instType": "SPOT",
                    "instId": "BTC-USDT"
                }]))
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_instruments("SPOT").await;
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.is_array());
        assert_eq!(json[0]["instId"], "BTC-USDT");
    }

    #[tokio::test]
    async fn test_get_ticker_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_ticker()
            .returning(|_: &str| Box::pin(async {
                Ok(OkxTicker {
                    inst_id: "BTC-USDT".into(),
                    last: "45200.0".into(),
                    last_sz: "1.5".into(),
                    ask_px: "45210.0".into(),
                    bid_px: "45190.0".into(),
                    open_24h: "44800.0".into(),
                    high_24h: "46000.0".into(),
                    low_24h: "44500.0".into(),
                    vol_ccy_24h: "150000000".into(),
                    vol_24h: "3333.3".into(),
                    sod_utc0: "44900.0".into(),
                    sod_utc8: "45000.0".into(),
                    ts: "1597026383085".into(),
                })
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_ticker("BTC-USDT").await;
        assert!(result.is_ok());
        let ticker = result.unwrap();
        assert_eq!(ticker.inst_id, "BTC-USDT");
        assert_eq!(ticker.last, "45200.0");
    }

    #[tokio::test]
    async fn test_get_funding_rate_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_funding_rate()
            .returning(|_: &str| Box::pin(async {
                Ok(OkxFundingRate {
                    inst_id: "BTC-USDT-SWAP".into(),
                    funding_rate: "0.0001".into(),
                    next_funding_rate: "0.00015".into(),
                    funding_time: "1597026383085".into(),
                    inst_type: "SWAP".into(),
                })
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_funding_rate("BTC-USDT-SWAP").await;
        assert!(result.is_ok());
        let fr = result.unwrap();
        assert_eq!(fr.inst_id, "BTC-USDT-SWAP");
        assert_eq!(fr.funding_rate, "0.0001");
    }

    #[tokio::test]
    async fn test_get_mark_price_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_mark_price()
            .returning(|_: &str| Box::pin(async {
                Ok(OkxMarkPrice {
                    inst_id: "BTC-USDT".into(),
                    mark_px: "45200.0".into(),
                    ts: "1597026383085".into(),
                })
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_mark_price("BTC-USDT").await;
        assert!(result.is_ok());
        let mp = result.unwrap();
        assert_eq!(mp.inst_id, "BTC-USDT");
        assert_eq!(mp.mark_px, "45200.0");
    }

    #[tokio::test]
    async fn test_get_index_price_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_index_price()
            .returning(|_: &str| Box::pin(async {
                Ok(OkxIndexPrice {
                    inst_id: "BTC-USDT".into(),
                    idx_px: "45205.0".into(),
                    ts: "1597026383085".into(),
                })
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_index_price("BTC-USDT").await;
        assert!(result.is_ok());
        let ip = result.unwrap();
        assert_eq!(ip.inst_id, "BTC-USDT");
        assert_eq!(ip.idx_px, "45205.0");
    }

    #[tokio::test]
    async fn test_get_open_interest_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_open_interest()
            .returning(|_: &str| Box::pin(async {
                Ok(OkxOpenInterest {
                    inst_id: "BTC-USDT".into(),
                    oi: "50000".into(),
                    oi_ccy: "45000".into(),
                    ts: "1597026383085".into(),
                })
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_open_interest("BTC-USDT").await;
        assert!(result.is_ok());
        let oi = result.unwrap();
        assert_eq!(oi.inst_id, "BTC-USDT");
        assert_eq!(oi.oi, "50000");
    }

    #[tokio::test]
    async fn test_get_trades_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_trades()
            .returning(|_: &str, _: Option<u32>| Box::pin(async {
                Ok(vec![
                    OkxTrade {
                        inst_id: "BTC-USDT".into(),
                        trade_id: "123456".into(),
                        px: "45200.0".into(),
                        sz: "0.5".into(),
                        side: "buy".into(),
                        ts: "1597026383085".into(),
                    },
                    OkxTrade {
                        inst_id: "BTC-USDT".into(),
                        trade_id: "123457".into(),
                        px: "45210.0".into(),
                        sz: "0.3".into(),
                        side: "sell".into(),
                        ts: "1597026383086".into(),
                    },
                ])
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_trades("BTC-USDT", Some(2)).await;
        assert!(result.is_ok());
        let trades = result.unwrap();
        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].trade_id, "123456");
    }

    #[tokio::test]
    async fn test_get_order_book_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_order_book()
            .returning(|_: &str, _: Option<u32>| Box::pin(async {
                Ok(OkxOrderBook {
                    asks: vec![
                        vec!["45210.0".into(), "1.0".into(), "0".into(), "1".into()],
                        vec!["45220.0".into(), "2.0".into(), "0".into(), "1".into()],
                    ],
                    bids: vec![
                        vec!["45190.0".into(), "1.5".into(), "0".into(), "1".into()],
                        vec!["45180.0".into(), "2.5".into(), "0".into(), "1".into()],
                    ],
                    ts: "1597026383085".into(),
                })
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_order_book("BTC-USDT", Some(2)).await;
        assert!(result.is_ok());
        let ob = result.unwrap();
        assert_eq!(ob.asks.len(), 2);
        assert_eq!(ob.bids.len(), 2);
        assert_eq!(ob.asks[0][0], "45210.0");
    }

    #[tokio::test]
    async fn test_get_announcements_with_mock() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_announcements()
            .returning(|| Box::pin(async {
                Ok(vec![
                    okx::api::announcements::announcements_api::AnnouncementPage {
                        details: vec![
                            okx::api::announcements::announcements_api::AnnouncementDetail {
                                ann_type: "delisting".into(),
                                p_time: "1597026383085".into(),
                                title: "Test Announcement".into(),
                                url: "https://www.okx.com/support/announcement/test".into(),
                            },
                        ],
                        total_page: "1".into(),
                    },
                ])
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_announcements().await;
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.is_array());
    }

    // ── API error propagation tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_get_balance_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_account_balance()
            .returning(|_| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_balance(None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_positions_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_positions()
            .returning(|_| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_positions(None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_place_order_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_place_order()
            .returning(|_: OkxPlaceOrderRequest| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let request = OkxPlaceOrderRequest {
            inst_id: "BTC-USDT".into(),
            td_mode: "cash".into(),
            side: "buy".into(),
            ord_type: "market".into(),
            sz: "1".into(),
            px: None,
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
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_cancel_order_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_cancel_order()
            .returning(|_: &str, _: &str| Box::pin(async { Err(quant_common::Error::Internal("cancel failed".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.cancel_order("BTC-USDT", "ord123").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("cancel failed")
        ));
    }

    #[tokio::test]
    async fn test_get_candles_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_candles()
            .returning(|_: &str, _: &str, _: Option<u32>| Box::pin(async {
                Err(quant_common::Error::Internal("api error".into()))
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_candles("BTC-USDT", "1m", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_instruments_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_instruments()
            .returning(|_: &str| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_instruments("SPOT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_ticker_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_ticker()
            .returning(|_: &str| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_ticker("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_funding_rate_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_funding_rate()
            .returning(|_: &str| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_funding_rate("BTC-USDT-SWAP").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_mark_price_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_mark_price()
            .returning(|_: &str| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_mark_price("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_index_price_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_index_price()
            .returning(|_: &str| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_index_price("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_open_interest_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_open_interest()
            .returning(|_: &str| Box::pin(async { Err(quant_common::Error::Internal("api error".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_open_interest("BTC-USDT").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_trades_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_trades()
            .returning(|_: &str, _: Option<u32>| Box::pin(async {
                Err(quant_common::Error::Internal("api error".into()))
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_trades("BTC-USDT", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_order_book_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_order_book()
            .returning(|_: &str, _: Option<u32>| Box::pin(async {
                Err(quant_common::Error::Internal("api error".into()))
            }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_order_book("BTC-USDT", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("api error")
        ));
    }

    #[tokio::test]
    async fn test_get_announcements_api_error() {
        let mut mock = MockOkxClient::new();
        mock.expect_get_announcements()
            .returning(|| Box::pin(async { Err(quant_common::Error::Internal("fetch failed".into())) }));
        let svc = make_service_with_mock(mock);
        let result = svc.get_announcements().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ServiceError::OkxApi(msg) if msg.contains("fetch failed")
        ));
    }
}
