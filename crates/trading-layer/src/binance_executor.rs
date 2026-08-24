//! Binance 订单执行器。
//!
//! [`BinanceExecutor`] wraps a shared Binance [`ClientInterface`] and
//! translates app-domain orders (`BTC-USDT` / `BTC/USDT`) into Binance
//! place-order requests (`BTCUSDT`), then normalizes the exchange response
//! into [`OrderDetails`].
//!
//! Binance order responses carry no fee field, so [`OrderDetails::fee`] is
//! surfaced as `Some(Zero)` and commission is resolved downstream.

use crate::live_exchange::{LiveExchange, OrderDetails};
use exchange_binance::types::{
    to_binance_symbol, BinanceOrder, BinanceOrderType, BinancePlaceOrderRequest, BinanceSide,
};
use exchange_binance::ClientInterface;
use quant_common::types::{Order, OrderSide, OrderStatus, OrderType};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument};

/// Convert an app-domain symbol (`BTC-USDT` or `BTC/USDT`) to Binance's
/// concatenated form (`BTCUSDT`).
///
/// `to_binance_symbol` only strips `-`; the app domain also uses `/`
/// separators (e.g. `BTC/USDT`), so normalise separators first.
fn binance_symbol(domain: &str) -> String {
    to_binance_symbol(&domain.replace('/', "-"))
}

/// Binance 订单执行器
#[derive(Clone)]
pub struct BinanceExecutor {
    client: Arc<RwLock<dyn ClientInterface>>,
}

impl BinanceExecutor {
    /// 创建新的 Binance 执行器
    pub fn new(client: Arc<RwLock<dyn ClientInterface>>) -> Self {
        Self { client }
    }

    /// 执行订单到 Binance
    #[instrument(skip(self), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side))]
    pub async fn execute_order(&self, order: &Order) -> Result<String> {
        let client = self.client.read().await;

        // 将内部订单转换为 Binance 下单请求
        let request = self.convert_order_to_binance(order)?;

        info!(
            "Placing order on Binance: {:?} {} {} @ {:?}",
            request.side, request.quantity, request.symbol, request.price
        );

        // 提交订单到 Binance
        let binance_order = client.place_order(&request).await?;

        info!(
            "Binance order placed successfully: {}",
            binance_order.order_id
        );

        Ok(binance_order.order_id.to_string())
    }

    /// 取消 Binance 订单
    #[instrument(skip(self), fields(symbol = %symbol, order_id = %order_id))]
    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<()> {
        let client = self.client.read().await;
        let order_id_i64 = order_id
            .parse::<i64>()
            .map_err(|_| Error::Validation(format!("Invalid Binance order id: {}", order_id)))?;
        client
            .cancel_order(&binance_symbol(symbol), order_id_i64)
            .await?;
        info!("Cancelled Binance order: {} on {}", order_id, symbol);
        Ok(())
    }

    /// 将内部订单转换为 Binance 下单请求
    fn convert_order_to_binance(&self, order: &Order) -> Result<BinancePlaceOrderRequest> {
        // 转换订单方向
        let side = match order.side {
            OrderSide::Buy => BinanceSide::Buy,
            OrderSide::Sell => BinanceSide::Sell,
        };

        // 转换订单类型
        let order_type = match order.order_type {
            OrderType::Limit => BinanceOrderType::Limit,
            OrderType::Market => BinanceOrderType::Market,
            OrderType::StopLoss
            | OrderType::StopLimit
            | OrderType::TWAP
            | OrderType::VWAP
            | OrderType::Iceberg => {
                return Err(Error::Validation(format!(
                    "Order type {:?} not supported by Binance executor",
                    order.order_type
                )));
            }
        };

        Ok(BinancePlaceOrderRequest {
            symbol: binance_symbol(&order.symbol),
            side,
            order_type,
            price: order.price,
            quantity: order.quantity,
            strategy_id: order.strategy_id.clone().into(),
        })
    }

    /// 查询订单成交明细（成交价、成交量、状态等）
    #[instrument(skip(self), fields(symbol = %symbol, order_id = %order_id))]
    pub async fn get_order_details(&self, symbol: &str, order_id: &str) -> Result<OrderDetails> {
        let client = self.client.read().await;
        let order_id_i64 = order_id
            .parse::<i64>()
            .map_err(|_| Error::Validation(format!("Invalid Binance order id: {}", order_id)))?;
        let binance_order = client
            .get_order(&binance_symbol(symbol), order_id_i64)
            .await?;
        Ok(Self::to_order_details(binance_order))
    }

    /// 将原始 Binance 订单归一化为 [`OrderDetails`]。
    ///
    /// 均价 = 累计成交额 / 已成交量（防除零；无成交时均价为 0）。
    pub(crate) fn to_order_details(o: BinanceOrder) -> OrderDetails {
        let avg_price = if o.executed_qty.is_zero() {
            Decimal::ZERO
        } else {
            o.cummulative_quote_qty / o.executed_qty
        };
        OrderDetails {
            avg_price,
            filled_quantity: o.executed_qty,
            status: Self::map_binance_status(&o.status).unwrap_or(OrderStatus::Filled),
            // BinanceOrder 无 fee 字段：本阶段固定为 0，由下游按配置费率兜底估算。
            fee: Some(Decimal::ZERO),
        }
    }

    pub(crate) fn map_binance_status(status: &str) -> Result<OrderStatus> {
        match status {
            "NEW" => Ok(OrderStatus::Submitted),
            "PARTIALLY_FILLED" => Ok(OrderStatus::PartiallyFilled),
            "FILLED" => Ok(OrderStatus::Filled),
            "CANCELED" => Ok(OrderStatus::Cancelled),
            "REJECTED" => Ok(OrderStatus::Rejected),
            "EXPIRED" => Ok(OrderStatus::Expired),
            _ => Err(Error::Trading(format!(
                "Unknown Binance order status: {}",
                status
            ))),
        }
    }
}

#[async_trait::async_trait]
impl LiveExchange for BinanceExecutor {
    async fn execute_order(&self, order: &Order) -> Result<String> {
        BinanceExecutor::execute_order(self, order).await
    }

    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<()> {
        BinanceExecutor::cancel_order(self, symbol, order_id).await
    }

    async fn get_order_details(&self, symbol: &str, order_id: &str) -> Result<OrderDetails> {
        BinanceExecutor::get_order_details(self, symbol, order_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_exchange::LiveExchange;
    use exchange_binance::MockBinanceClient;
    use quant_common::types::{Order, OrderSide, OrderType};
    use rust_decimal_macros::dec;
    use std::sync::Mutex;

    /// 构造一个假 Binance 订单返回体。
    fn binance_order(
        order_id: i64,
        status: &str,
        executed_qty: Decimal,
        cummulative_quote_qty: Decimal,
    ) -> BinanceOrder {
        BinanceOrder {
            symbol: "BTCUSDT".to_string(),
            order_id,
            client_order_id: "client-x".to_string(),
            status: status.to_string(),
            executed_qty,
            cummulative_quote_qty,
            price: dec!(10000),
            side: "BUY".to_string(),
            order_type: "LIMIT".to_string(),
            orig_qty: dec!(2),
            time: 0,
            update_time: 0,
        }
    }

    fn sample_order(
        order_type: OrderType,
        side: OrderSide,
        symbol: &str,
        price: Option<Decimal>,
    ) -> Order {
        Order { order_id: 1001,
        strategy_id: "test".to_string(),
        symbol: symbol.to_string(),
        order_type,
        side,
        price,
        quantity: dec!(2),
        filled_quantity: Decimal::ZERO,
        status: OrderStatus::Submitted,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        commission: Decimal::ZERO,
        slippage: Decimal::ZERO, exchange: "live".to_string(), }
    }

    fn make_executor(mock: MockBinanceClient) -> BinanceExecutor {
        let client: Arc<RwLock<dyn ClientInterface>> = Arc::new(RwLock::new(mock));
        BinanceExecutor::new(client)
    }

    #[tokio::test]
    async fn test_execute_order_converts_symbol_side_type_price_and_quantity() {
        let mut mock = MockBinanceClient::new();
        let captured = Arc::new(Mutex::new(None));
        let cap = captured.clone();
        mock.expect_place_order().returning(move |req| {
            *cap.lock().unwrap() = Some(req.clone());
            Box::pin(async move { Ok(binance_order(456, "NEW", Decimal::ZERO, Decimal::ZERO)) })
        });

        let executor = make_executor(mock);
        let order = sample_order(
            OrderType::Limit,
            OrderSide::Buy,
            "BTC-USDT",
            Some(dec!(50000)),
        );

        let order_id = executor.execute_order(&order).await.unwrap();
        assert_eq!(order_id, "456");

        let req = captured
            .lock()
            .unwrap()
            .clone()
            .expect("place_order called");
        assert_eq!(req.symbol, "BTCUSDT");
        assert!(matches!(req.side, BinanceSide::Buy));
        assert!(matches!(req.order_type, BinanceOrderType::Limit));
        assert_eq!(req.price, Some(dec!(50000)));
        assert_eq!(req.quantity, dec!(2));
    }

    #[tokio::test]
    async fn test_execute_order_handles_slash_domain_symbol() {
        let mut mock = MockBinanceClient::new();
        let captured = Arc::new(Mutex::new(None));
        let cap = captured.clone();
        mock.expect_place_order().returning(move |req| {
            *cap.lock().unwrap() = Some(req.clone());
            Box::pin(async move { Ok(binance_order(1, "NEW", Decimal::ZERO, Decimal::ZERO)) })
        });

        let executor = make_executor(mock);
        let order = sample_order(
            OrderType::Limit,
            OrderSide::Sell,
            "BTC/USDT",
            Some(dec!(50000)),
        );

        executor.execute_order(&order).await.unwrap();
        let req = captured
            .lock()
            .unwrap()
            .clone()
            .expect("place_order called");
        assert_eq!(req.symbol, "BTCUSDT");
        assert!(matches!(req.side, BinanceSide::Sell));
    }

    #[tokio::test]
    async fn test_execute_order_uses_market_without_price() {
        let mut mock = MockBinanceClient::new();
        let captured = Arc::new(Mutex::new(None));
        let cap = captured.clone();
        mock.expect_place_order().returning(move |req| {
            *cap.lock().unwrap() = Some(req.clone());
            Box::pin(async move { Ok(binance_order(7, "NEW", Decimal::ZERO, Decimal::ZERO)) })
        });

        let executor = make_executor(mock);
        let order = sample_order(OrderType::Market, OrderSide::Buy, "BTC-USDT", None);

        executor.execute_order(&order).await.unwrap();
        let req = captured
            .lock()
            .unwrap()
            .clone()
            .expect("place_order called");
        assert!(matches!(req.order_type, BinanceOrderType::Market));
        assert!(req.price.is_none());
    }

    #[tokio::test]
    async fn test_execute_order_rejects_unsupported_order_type() {
        let mut mock = MockBinanceClient::new();
        mock.expect_place_order().times(0);
        let executor = make_executor(mock);
        let order = sample_order(OrderType::TWAP, OrderSide::Buy, "BTC-USDT", None);

        let err = executor
            .execute_order(&order)
            .await
            .expect_err("must error");
        assert!(err
            .to_string()
            .contains("not supported by Binance executor"));
    }

    #[tokio::test]
    async fn test_get_order_details_derives_avg_price_and_status() {
        let mut mock = MockBinanceClient::new();
        mock.expect_get_order().returning(|symbol, order_id| {
            assert_eq!(symbol, "BTCUSDT");
            assert_eq!(order_id, 456);
            Box::pin(async move { Ok(binance_order(456, "FILLED", dec!(2), dec!(20010))) })
        });

        let executor = make_executor(mock);
        let d = executor.get_order_details("BTC-USDT", "456").await.unwrap();

        assert_eq!(d.avg_price, dec!(10005)); // 20010 / 2
        assert_eq!(d.filled_quantity, dec!(2));
        assert_eq!(d.status, OrderStatus::Filled);
        assert_eq!(d.fee, Some(Decimal::ZERO));
    }

    #[tokio::test]
    async fn test_get_order_details_guards_division_by_zero() {
        let mut mock = MockBinanceClient::new();
        mock.expect_get_order().returning(|_, _| {
            Box::pin(async move { Ok(binance_order(1, "NEW", Decimal::ZERO, Decimal::ZERO)) })
        });

        let executor = make_executor(mock);
        let d = executor.get_order_details("BTC-USDT", "1").await.unwrap();

        assert_eq!(d.avg_price, Decimal::ZERO);
        assert_eq!(d.filled_quantity, Decimal::ZERO);
        assert_eq!(d.status, OrderStatus::Submitted); // "NEW" -> Submitted
    }

    #[test]
    fn test_map_binance_status_known_and_unknown() {
        assert_eq!(
            BinanceExecutor::map_binance_status("NEW").unwrap(),
            OrderStatus::Submitted
        );
        assert_eq!(
            BinanceExecutor::map_binance_status("PARTIALLY_FILLED").unwrap(),
            OrderStatus::PartiallyFilled
        );
        assert_eq!(
            BinanceExecutor::map_binance_status("FILLED").unwrap(),
            OrderStatus::Filled
        );
        assert_eq!(
            BinanceExecutor::map_binance_status("CANCELED").unwrap(),
            OrderStatus::Cancelled
        );
        assert!(BinanceExecutor::map_binance_status("UNKNOWN").is_err());
    }

    #[tokio::test]
    async fn test_cancel_order_converts_symbol_and_parses_id() {
        let mut mock = MockBinanceClient::new();
        mock.expect_cancel_order().returning(|symbol, order_id| {
            assert_eq!(symbol, "BTCUSDT");
            assert_eq!(order_id, 456);
            Box::pin(async move { Ok(()) })
        });

        let executor = make_executor(mock);
        executor.cancel_order("BTC/USDT", "456").await.unwrap();
    }

    #[tokio::test]
    async fn test_live_exchange_seam_drives_binance_executor() {
        // 验证 BinanceExecutor 可作为 Arc<dyn LiveExchange> 使用。
        let mut mock = MockBinanceClient::new();
        mock.expect_place_order().returning(|_| {
            Box::pin(async move { Ok(binance_order(9, "NEW", Decimal::ZERO, Decimal::ZERO)) })
        });

        let client: Arc<RwLock<dyn ClientInterface>> = Arc::new(RwLock::new(mock));
        let executor: Arc<dyn LiveExchange> = Arc::new(BinanceExecutor::new(client));
        let order = sample_order(
            OrderType::Limit,
            OrderSide::Buy,
            "BTC-USDT",
            Some(dec!(10000)),
        );

        let order_id = executor.execute_order(&order).await.unwrap();
        assert_eq!(order_id, "9");
    }
}
