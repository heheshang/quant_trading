//! 执行引擎与执行策略单元测试。

use super::*;
use crate::binance_executor::BinanceExecutor;
use crate::order_manager::OrderManager;
use chrono::Utc;
use exchange_binance::types::BinanceOrder;
use exchange_binance::MockBinanceClient;
use quant_common::config::TradingConfig;
use quant_common::types::{MarketData, Order, OrderSide, OrderStatus, OrderType};
use quant_common::Error;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

/// 构造测试用交易配置。
fn config(paper: bool, delay_ms: u64, slippage: f64, commission: f64) -> TradingConfig {
    TradingConfig {
        enable_paper_trading: paper,
        max_orders_per_second: 10,
        default_commission_rate: commission,
        default_slippage: slippage,
        order_timeout_seconds: 30,
        simulation_delay_ms: delay_ms,
    }
}

/// 构造测试用订单。
fn sample_order(side: OrderSide, price: Option<Decimal>) -> Order {
    Order { order_id: 1001,
    strategy_id: "test".to_string(),
    symbol: "BTC/USDT".to_string(),
    order_type: OrderType::Limit,
    side,
    price,
    quantity: dec!(2),
    filled_quantity: Decimal::ZERO,
    status: OrderStatus::Submitted,
    created_at: Utc::now(),
    updated_at: Utc::now(),
    commission: Decimal::ZERO,
    slippage: Decimal::ZERO, exchange: "paper".to_string(), }
}

/// 构造测试用行情数据（买卖价一致，避免 spread 影响）。
fn sample_market(close: Decimal) -> MarketData {
    MarketData {
        symbol: "BTC/USDT".to_string(),
        timestamp: Utc::now(),
        open: close,
        high: close,
        low: close,
        close,
        volume: dec!(100),
        turnover: dec!(1_000_000),
        open_interest: None,
        bid_prices: vec![close],
        bid_volumes: vec![dec!(1)],
        ask_prices: vec![close],
        ask_volumes: vec![dec!(1)],
    }
}

/// 模拟真实调用链：先 `submit_order` 入账，再用返回的 order_id 执行。
async fn submit_and_execute(
    engine: &ExecutionEngine,
    order_manager: &OrderManager,
    order: Order,
    market_data: &MarketData,
) -> quant_common::Result<ExecutionResult> {
    let order_id = order_manager.submit_order(order.clone()).await?;
    let mut order = order;
    order.order_id = order_id;
    engine.execute_order(order, market_data).await
}

/// 记录执行事件的回调。
struct RecordingCallback {
    results: Arc<Mutex<Vec<ExecutionResult>>>,
}

#[async_trait::async_trait]
impl ExecutionCallback for RecordingCallback {
    async fn on_order_executed(&self, result: &ExecutionResult) {
        self.results.lock().unwrap().push(result.clone());
    }
}

#[tokio::test]
async fn test_paper_buy_applies_slippage_and_commission() {
    let order_manager = Arc::new(OrderManager::new());
    let engine =
        ExecutionEngine::new_binance(order_manager.clone(), config(true, 0, 0.0005, 0.001), None);

    let result = submit_and_execute(
        &engine,
        &order_manager,
        sample_order(OrderSide::Buy, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();

    assert_eq!(result.filled_quantity, dec!(2));
    assert_eq!(result.status, OrderStatus::Filled);
    // 买入溢价：10000 + 10000 * 0.0005 = 10005
    assert!((result.avg_price.to_f64().unwrap() - 10005.0).abs() < 0.01);
    // 佣金 = 10005 * 2 * 0.001 = 20.01
    assert!((result.commission.to_f64().unwrap() - 20.01).abs() < 0.01);
}

#[tokio::test]
async fn test_paper_sell_applies_discount() {
    let order_manager = Arc::new(OrderManager::new());
    let engine =
        ExecutionEngine::new_binance(order_manager.clone(), config(true, 0, 0.0005, 0.0), None);

    let result = submit_and_execute(
        &engine,
        &order_manager,
        sample_order(OrderSide::Sell, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();

    // 卖出折价：10000 - 10000 * 0.0005 = 9995
    assert!((result.avg_price.to_f64().unwrap() - 9995.0).abs() < 0.01);
    assert_eq!(result.commission, Decimal::ZERO);
}

#[tokio::test]
async fn test_paper_zero_slippage_and_zero_commission() {
    let order_manager = Arc::new(OrderManager::new());
    let engine =
        ExecutionEngine::new_binance(order_manager.clone(), config(true, 0, 0.0, 0.0), None);

    let result = submit_and_execute(
        &engine,
        &order_manager,
        sample_order(OrderSide::Buy, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();

    assert_eq!(result.avg_price, dec!(10000));
    assert_eq!(result.commission, Decimal::ZERO);
}

#[tokio::test]
async fn test_paper_delay_still_fills_order() {
    let order_manager = Arc::new(OrderManager::new());
    let engine =
        ExecutionEngine::new_binance(order_manager.clone(), config(true, 5, 0.0005, 0.001), None);

    let result = submit_and_execute(
        &engine,
        &order_manager,
        sample_order(OrderSide::Buy, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();

    assert_eq!(result.status, OrderStatus::Filled);
    assert_eq!(result.filled_quantity, dec!(2));
}

#[tokio::test]
async fn test_callback_fires_on_success_only() {
    let results: Arc<Mutex<Vec<ExecutionResult>>> = Arc::new(Mutex::new(Vec::new()));
    let order_manager = Arc::new(OrderManager::new());
    let mut engine =
        ExecutionEngine::new_binance(order_manager.clone(), config(true, 0, 0.0005, 0.001), None);
    engine.register_callback(Box::new(RecordingCallback {
        results: results.clone(),
    }));

    // 成功路径：paper 模式
    let _ = submit_and_execute(
        &engine,
        &order_manager,
        sample_order(OrderSide::Buy, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();
    assert_eq!(results.lock().unwrap().len(), 1);

    // 失败路径：real 模式且无 binance executor → 不触发回调
    let failing_order_manager = Arc::new(OrderManager::new());
    let mut failing = ExecutionEngine::new_binance(
        failing_order_manager.clone(),
        config(false, 0, 0.0005, 0.001),
        None,
    );
    failing.register_callback(Box::new(RecordingCallback {
        results: results.clone(),
    }));
    let _ = failing
        .execute_order(
            sample_order(OrderSide::Sell, Some(dec!(10000))),
            &sample_market(dec!(10000)),
        )
        .await
        .expect_err("expected error");

    // 失败不增加回调
    assert_eq!(results.lock().unwrap().len(), 1);
}

/// 构造一个 Binance 订单返回体，用于 mock 的 `place_order`/`get_order` 返回。
fn binance_order(
    order_id: i64,
    status: &str,
    executed_qty: Decimal,
    cummulative_quote_qty: Decimal,
) -> BinanceOrder {
    BinanceOrder {
        symbol: "BTCUSDT".to_string(),
        order_id,
        client_order_id: "client-bin".to_string(),
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

/// 用 mock Binance 客户端构造一个以 Binance 为实盘执行器的引擎。
fn binance_engine(order_manager: Arc<OrderManager>, mock: MockBinanceClient) -> ExecutionEngine {
    let client: Arc<RwLock<dyn exchange_binance::ClientInterface>> = Arc::new(RwLock::new(mock));
    let executor = BinanceExecutor::new(client);
    ExecutionEngine::new_binance(
        order_manager,
        config(false, 0, 0.0005, 0.001),
        Some(Arc::new(executor)),
    )
}

#[tokio::test]
async fn test_binance_real_success_uses_computed_avg_price() {
    let mut mock = MockBinanceClient::new();
    mock.expect_place_order().returning(|_| {
        Box::pin(async move { Ok(binance_order(456, "NEW", Decimal::ZERO, Decimal::ZERO)) })
    });
    mock.expect_get_order().returning(|_, _| {
        Box::pin(async move { Ok(binance_order(456, "FILLED", dec!(2), dec!(20010))) })
    });

    let order_manager = Arc::new(OrderManager::new());
    let engine = binance_engine(order_manager.clone(), mock);

    let result = submit_and_execute(
        &engine,
        &order_manager,
        sample_order(OrderSide::Buy, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();

    // 均价 = 20010 / 2 = 10005
    assert_eq!(result.avg_price, dec!(10005));
    assert_eq!(result.filled_quantity, dec!(2));
    assert_eq!(result.status, OrderStatus::Filled);
    // Binance 无 fee 字段，BinanceExecutor 返回 fee=0 → 手续费为 0
    assert_eq!(result.commission, Decimal::ZERO);
}

#[tokio::test]
async fn test_binance_real_fallback_on_details_error() {
    let mut mock = MockBinanceClient::new();
    mock.expect_place_order().returning(|_| {
        Box::pin(async move { Ok(binance_order(456, "NEW", Decimal::ZERO, Decimal::ZERO)) })
    });
    mock.expect_get_order().returning(|_, _| {
        Box::pin(async move { Err(Error::Trading("order not found".to_string())) })
    });

    let order_manager = Arc::new(OrderManager::new());
    let engine = binance_engine(order_manager.clone(), mock);

    let result = submit_and_execute(
        &engine,
        &order_manager,
        sample_order(OrderSide::Buy, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();

    // 查询失败：保守回退（订单价、全额成交、Filled）；手续费按配置费率估算
    assert_eq!(result.avg_price, dec!(10000));
    assert_eq!(result.filled_quantity, dec!(2));
    assert_eq!(result.status, OrderStatus::Filled);
    assert!((result.commission.to_f64().unwrap() - 20.0).abs() < 0.01);
}

#[tokio::test]
async fn test_binance_without_executor_errors() {
    let order_manager = Arc::new(OrderManager::new());
    let engine =
        ExecutionEngine::new_binance(order_manager.clone(), config(false, 0, 0.0005, 0.001), None);

    let err = engine
        .execute_order(
            sample_order(OrderSide::Buy, Some(dec!(10000))),
            &sample_market(dec!(10000)),
        )
        .await
        .expect_err("expected an error");

    assert!(err.to_string().contains("No Binance executor configured"));
}
