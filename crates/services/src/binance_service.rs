//! Binance exchange operations service.
//!
//! Mirrors `OkxService`: wraps the Binance REST client (behind a shared
//! `Option<Arc<dyn ClientInterface>>`) and exposes typed service methods that
//! map errors, convert symbols (`BTC-USDT` ⇄ `BTCUSDT`), and avoid leaking the
//! raw client to the command layer.

use crate::error::{ServiceError, ServiceResult};
use exchange_binance::types::*;
use exchange_binance::ClientInterface;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, instrument};

type SharedBinanceClient = Arc<RwLock<Option<Arc<dyn ClientInterface + Send + Sync>>>>;

/// Executes the body with the borrowed Binance client, or
/// `Err(BinanceNotInitialized)` when the client is absent.
macro_rules! with_binance_client {
    ($self:expr, |$client:ident| $body:expr) => {{
        let guard = $self.binance_client.read().await;
        match guard.as_ref() {
            Some(client_arc) => {
                let $client = client_arc.as_ref();
                $body
            }
            None => Err(ServiceError::BinanceNotInitialized),
        }
    }};
}

/// Binance exchange operations.
pub struct BinanceService {
    binance_client: SharedBinanceClient,
}

impl BinanceService {
    pub fn new(binance_client: SharedBinanceClient) -> Self {
        Self { binance_client }
    }

    /// Account balances across assets.
    pub async fn get_balance(&self) -> ServiceResult<Vec<BinanceBalance>> {
        with_binance_client!(self, |client| {
            client
                .get_account_balance()
                .await
                .map_err(|e| ServiceError::BinanceApi(e.to_string()))
        })
    }

    /// Klines for a domain symbol (e.g. `BTC-USDT`).
    #[instrument(skip(self), fields(symbol = %symbol))]
    pub async fn get_candles(
        &self,
        symbol: &str,
        interval: &str,
        limit: Option<u32>,
    ) -> ServiceResult<Vec<BinanceKline>> {
        let binance_symbol = to_binance_symbol(symbol);
        with_binance_client!(self, |client| {
            client
                .get_candles(&binance_symbol, interval, limit)
                .await
                .map_err(|e| ServiceError::BinanceApi(e.to_string()))
        })
    }

    /// Order-book depth for a domain symbol.
    pub async fn get_order_book(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> ServiceResult<BinanceOrderBook> {
        let binance_symbol = to_binance_symbol(symbol);
        with_binance_client!(self, |client| {
            client
                .get_order_book(&binance_symbol, limit)
                .await
                .map_err(|e| ServiceError::BinanceApi(e.to_string()))
        })
    }

    /// Place an order (domain symbol).
    #[instrument(skip(self, request), fields(symbol = %request.symbol))]
    pub async fn place_order(&self, request: BinancePlaceOrderRequest) -> ServiceResult<BinanceOrder> {
        let req = BinancePlaceOrderRequest {
            symbol: to_binance_symbol(&request.symbol),
            ..request
        };
        with_binance_client!(self, |client| {
            client.place_order(&req).await.map_err(|e| {
                error!("Place Binance order failed: {}", e);
                ServiceError::BinanceApi(e.to_string())
            })
        })
    }

    /// Cancel an order (domain symbol).
    pub async fn cancel_order(&self, symbol: &str, order_id: i64) -> ServiceResult<()> {
        let binance_symbol = to_binance_symbol(symbol);
        with_binance_client!(self, |client| {
            client
                .cancel_order(&binance_symbol, order_id)
                .await
                .map_err(|e| ServiceError::BinanceApi(e.to_string()))
        })
    }

    /// Connection status for the UI.
    pub async fn check_status(&self) -> ServiceResult<serde_json::Value> {
        let guard = self.binance_client.read().await;
        Ok(serde_json::json!({ "connected": guard.is_some() }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exchange_binance::MockBinanceClient;
    use rust_decimal::Decimal;

    fn service_with(client: MockBinanceClient) -> BinanceService {
        let shared: SharedBinanceClient = Arc::new(RwLock::new(Some(Arc::new(client))));
        BinanceService::new(shared)
    }

    #[tokio::test]
    async fn check_status_reflects_connected() {
        let service = service_with(MockBinanceClient::new());
        let status = service.check_status().await.expect("status");
        assert_eq!(status["connected"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn get_balance_returns_client_data() {
        let mut client = MockBinanceClient::new();
        client.expect_get_account_balance().returning(|| {
            Box::pin(async {
                Ok(vec![BinanceBalance {
                    asset: "USDT".to_string(),
                    free: Decimal::new(10000, 2),
                    locked: Decimal::ZERO,
                }])
            })
        });
        let service = service_with(client);
        let balances = service.get_balance().await.expect("balance");
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].asset, "USDT");
    }

    #[tokio::test]
    async fn get_candles_converts_symbol() {
        let mut client = MockBinanceClient::new();
        client.expect_get_candles().withf(|s, _, _| s == "BTCUSDT").returning(
            |_, _, _| {
                Box::pin(async {
                    Ok(vec![BinanceKline {
                        open_time: 0,
                        open: Decimal::ZERO,
                        high: Decimal::ZERO,
                        low: Decimal::ZERO,
                        close: Decimal::ZERO,
                        volume: Decimal::ZERO,
                        close_time: 0,
                        quote_volume: Decimal::ZERO,
                        trades: 0,
                    }])
                })
            },
        );
        let service = service_with(client);
        let candles = service.get_candles("BTC-USDT", "1h", None).await.expect("candles");
        assert_eq!(candles.len(), 1);
    }

    #[tokio::test]
    async fn uninitialized_client_errors() {
        let service = BinanceService::new(Arc::new(RwLock::new(None)));
        let err = service.get_balance().await.expect_err("should fail");
        assert!(matches!(err, ServiceError::BinanceNotInitialized));
    }
}
