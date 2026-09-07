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
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

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
    Order {
        order_id: 1001,
        strategy_id: "test".to_string(),
        symbol: "BTC/USDT".to_string(),
        order_type: if price.is_some() {
            OrderType::Limit
        } else {
            OrderType::Market
        },
        side,
        price,
        quantity: dec!(2),
        filled_quantity: Decimal::ZERO,
        status: OrderStatus::Submitted,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        commission: Decimal::ZERO,
        slippage: Decimal::ZERO,
        exchange: "paper".to_string(),
    }
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
        self.results.lock().await.push(result.clone());
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
    // 限价单：成交价不得劣于限价——市价=限价=10000，滑点被 clamp 到 10000。
    assert!((result.avg_price.to_f64().unwrap() - 10000.0).abs() < 0.01);
    // 佣金 = 10000 * 2 * 0.001 = 20
    assert!((result.commission.to_f64().unwrap() - 20.0).abs() < 0.01);
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

    // 限价卖单：成交价不得劣于限价——市价=限价=10000，折价被 clamp 到 10000。
    assert!((result.avg_price.to_f64().unwrap() - 10000.0).abs() < 0.01);
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
    assert_eq!(results.lock().await.len(), 1);

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
    assert_eq!(results.lock().await.len(), 1);
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
    // 交易所缺失 fee，按真实成交额和配置费率估算。
    assert!((result.commission.to_f64().unwrap() - 20.01).abs() < 0.000001);
    let stored = order_manager.get_order(result.order_id).await.unwrap();
    assert_eq!(stored.filled_quantity, dec!(2));
    assert_eq!(stored.commission, result.commission);
}

#[tokio::test]
async fn test_binance_details_error_keeps_submitted_without_resubmission() {
    let mut mock = MockBinanceClient::new();
    mock.expect_place_order().times(1).returning(|_| {
        Box::pin(async move { Ok(binance_order(456, "NEW", Decimal::ZERO, Decimal::ZERO)) })
    });
    mock.expect_get_order().times(1).returning(|_, _| {
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

    assert_eq!(result.avg_price, Decimal::ZERO);
    assert_eq!(result.filled_quantity, Decimal::ZERO);
    assert_eq!(result.status, OrderStatus::Submitted);
    assert_eq!(result.commission, Decimal::ZERO);
    let stored = order_manager.get_order(result.order_id).await.unwrap();
    assert_eq!(stored.status, OrderStatus::Submitted);
    assert_eq!(stored.filled_quantity, Decimal::ZERO);
    assert_eq!(stored.commission, Decimal::ZERO);
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

    assert!(matches!(err, Error::Internal(_)));
}

#[tokio::test]
async fn test_cancelled_order_cannot_fill_after_execution_delay() {
    let manager = Arc::new(OrderManager::new());
    let mut engine =
        ExecutionEngine::new_binance(manager.clone(), config(true, 10, 0.0, 0.001), None);
    let callbacks = Arc::new(Mutex::new(Vec::new()));
    engine.register_callback(Box::new(RecordingCallback {
        results: callbacks.clone(),
    }));
    let id = manager
        .submit_order(sample_order(OrderSide::Buy, Some(dec!(10000))))
        .await
        .unwrap();
    let order = manager.get_order(id).await.unwrap();
    let market = sample_market(dec!(10000));
    let execution = engine.execute_order(order, &market);
    tokio::pin!(execution);
    // Poll execution into its delay before cancelling, without relying on wall-clock races.
    tokio::select! {
        biased;
        result = &mut execution => panic!("execution completed before delay: {result:?}"),
        _ = std::future::ready(()) => {}
    }
    manager.cancel_order(id).await.unwrap();
    assert!(matches!(execution.await, Err(Error::Trading(_))));
    let stored = manager.get_order(id).await.unwrap();
    assert_eq!(stored.status, OrderStatus::Cancelled);
    assert_eq!(stored.filled_quantity, Decimal::ZERO);
    assert_eq!(stored.commission, Decimal::ZERO);
    assert!(callbacks.lock().await.is_empty());
}

#[tokio::test]
async fn test_paper_calculation_leaves_memory_and_callbacks_unchanged() {
    let manager = Arc::new(OrderManager::new());
    let mut engine =
        ExecutionEngine::new_binance(manager.clone(), config(true, 0, 0.0, 0.001), None);
    let callbacks = Arc::new(Mutex::new(Vec::new()));
    engine.register_callback(Box::new(RecordingCallback {
        results: callbacks.clone(),
    }));
    let id = manager
        .submit_order(sample_order(OrderSide::Buy, Some(dec!(10000))))
        .await
        .unwrap();
    let result = engine
        .calculate_paper_fill(
            manager.get_order(id).await.unwrap(),
            &sample_market(dec!(9990)),
        )
        .await
        .unwrap();
    assert_eq!(result.status, OrderStatus::Filled);
    assert_eq!(result.avg_price, dec!(9990));
    let stored = manager.get_order(id).await.unwrap();
    assert_eq!(stored.status, OrderStatus::Submitted);
    assert_eq!(stored.filled_quantity, Decimal::ZERO);
    assert_eq!(stored.commission, Decimal::ZERO);
    assert!(callbacks.lock().await.is_empty());
    manager.apply_execution_result(&result).await.unwrap();
    manager.apply_execution_result(&result).await.unwrap();
    let stored = manager.get_order(id).await.unwrap();
    assert_eq!(stored.status, OrderStatus::Filled);
    assert_eq!(stored.filled_quantity, dec!(2));
    assert_eq!(stored.commission, result.commission);
    assert_eq!(stored.price, Some(dec!(10000)));
}

#[tokio::test]
async fn test_paper_recovered_order_can_fill_without_memory_entry() {
    let manager = Arc::new(OrderManager::new());
    let engine = ExecutionEngine::new_binance(manager.clone(), config(true, 0, 0.0, 0.0), None);
    let result = engine
        .fill_order(
            sample_order(OrderSide::Buy, Some(dec!(10000))),
            &sample_market(dec!(10000)),
        )
        .await
        .unwrap();
    assert_eq!(result.status, OrderStatus::Filled);
    assert_eq!(result.filled_quantity, dec!(2));
    assert!(matches!(
        manager.get_order(result.order_id).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn test_paper_limit_requires_market_touch_on_each_side() {
    let manager = Arc::new(OrderManager::new());
    let engine = ExecutionEngine::new_binance(manager.clone(), config(true, 0, 0.0, 0.0), None);
    for (side, untouched, touched) in [
        (OrderSide::Buy, dec!(101), dec!(100)),
        (OrderSide::Sell, dec!(99), dec!(100)),
    ] {
        let id = manager
            .submit_order(sample_order(side, Some(dec!(100))))
            .await
            .unwrap();
        let order = manager.get_order(id).await.unwrap();
        assert!(matches!(
            engine
                .fill_order(order.clone(), &sample_market(untouched))
                .await,
            Err(Error::Trading(_))
        ));
        assert_eq!(
            manager.get_order(id).await.unwrap().status,
            OrderStatus::Submitted
        );
        assert_eq!(
            engine
                .fill_order(order, &sample_market(touched))
                .await
                .unwrap()
                .status,
            OrderStatus::Filled
        );
    }
}

#[tokio::test]
async fn test_paper_rejects_invalid_price_and_quantity_before_publication() {
    let manager = Arc::new(OrderManager::new());
    let engine = ExecutionEngine::new_binance(manager, config(true, 0, 0.0, 0.0), None);
    for (quantity, limit, market_price) in [
        (dec!(0), dec!(100), dec!(100)),
        (dec!(-1), dec!(100), dec!(100)),
        (dec!(2), dec!(0), dec!(100)),
        (dec!(2), dec!(100), dec!(0)),
        (dec!(2), dec!(100), dec!(-1)),
    ] {
        let mut order = sample_order(OrderSide::Buy, Some(limit));
        order.quantity = quantity;
        assert!(matches!(
            engine
                .calculate_paper_fill(order, &sample_market(market_price))
                .await,
            Err(Error::Validation(_))
        ));
    }
    let mut missing_limit = sample_order(OrderSide::Buy, None);
    missing_limit.order_type = OrderType::Limit;
    assert!(matches!(
        engine
            .calculate_paper_fill(missing_limit, &sample_market(dec!(100)))
            .await,
        Err(Error::Validation(_))
    ));
}

#[tokio::test]
async fn test_partial_paper_fill_only_charges_remaining_quantity() {
    let manager = Arc::new(OrderManager::new());
    let engine = ExecutionEngine::new_binance(manager, config(true, 0, 0.0, 0.001), None);
    let mut order = sample_order(OrderSide::Buy, Some(dec!(10000)));
    order.status = OrderStatus::PartiallyFilled;
    order.filled_quantity = dec!(1);
    order.commission = dec!(7);
    let result = engine
        .calculate_paper_fill(order, &sample_market(dec!(9000)))
        .await
        .unwrap();
    assert_eq!(result.filled_quantity, dec!(2));
    assert!((result.commission.to_f64().unwrap() - 16.0).abs() < 0.000001);
}

#[tokio::test]
async fn terminal_execution_replay_rejects_changed_fee_or_quantity() {
    let manager = Arc::new(OrderManager::new());
    let engine = ExecutionEngine::new_binance(manager.clone(), config(true, 0, 0.0, 0.001), None);
    let mut order = sample_order(OrderSide::Buy, None);
    order.order_id = manager.submit_order(order.clone()).await.unwrap();
    let first = engine
        .fill_order(order.clone(), &sample_market(dec!(100)))
        .await
        .unwrap();
    manager.apply_execution_result(&first).await.unwrap();
    assert!(engine
        .fill_order(order, &sample_market(dec!(200)))
        .await
        .is_err());
    assert_eq!(
        manager.get_order(first.order_id).await.unwrap().commission,
        first.commission
    );

    let mut cancelled = sample_order(OrderSide::Buy, None);
    cancelled.order_id = manager.submit_order(cancelled.clone()).await.unwrap();
    manager
        .update_order_status(cancelled.order_id, OrderStatus::Cancelled)
        .await
        .unwrap();
    let mut replay = first;
    replay.order_id = cancelled.order_id;
    replay.status = OrderStatus::Cancelled;
    replay.commission = Decimal::ZERO;
    assert!(manager.apply_execution_result(&replay).await.is_err());
    assert_eq!(
        manager
            .get_order(cancelled.order_id)
            .await
            .unwrap()
            .filled_quantity,
        Decimal::ZERO
    );
}

#[tokio::test]
async fn test_all_terminal_statuses_reject_reactivation_and_allow_replay() {
    let manager = OrderManager::new();
    for terminal in [
        OrderStatus::Filled,
        OrderStatus::Cancelled,
        OrderStatus::Rejected,
        OrderStatus::Expired,
    ] {
        let id = manager
            .submit_order(sample_order(OrderSide::Buy, Some(dec!(100))))
            .await
            .unwrap();
        manager
            .update_order_status(id, terminal.clone())
            .await
            .unwrap();
        manager
            .update_order_status(id, terminal.clone())
            .await
            .unwrap();
        for next in [
            OrderStatus::Submitted,
            OrderStatus::PartiallyFilled,
            OrderStatus::Filled,
            OrderStatus::Cancelled,
        ] {
            if next != terminal {
                assert!(matches!(
                    manager.update_order_status(id, next).await,
                    Err(Error::Trading(_))
                ));
            }
        }
        assert_eq!(manager.get_order(id).await.unwrap().status, terminal);
    }
}

#[tokio::test]
async fn test_binance_partial_fill_preserves_active_quantity_and_estimated_fee() {
    let mut mock = MockBinanceClient::new();
    mock.expect_place_order().times(1).returning(|_| {
        Box::pin(async move { Ok(binance_order(456, "NEW", Decimal::ZERO, Decimal::ZERO)) })
    });
    mock.expect_get_order().times(1).returning(|_, _| {
        Box::pin(async move {
            Ok(binance_order(
                456,
                "PARTIALLY_FILLED",
                dec!(0.5),
                dec!(5000),
            ))
        })
    });
    let manager = Arc::new(OrderManager::new());
    let engine = binance_engine(manager.clone(), mock);
    let result = submit_and_execute(
        &engine,
        &manager,
        sample_order(OrderSide::Buy, Some(dec!(10000))),
        &sample_market(dec!(10000)),
    )
    .await
    .unwrap();
    assert_eq!(result.status, OrderStatus::PartiallyFilled);
    assert_eq!(result.filled_quantity, dec!(0.5));
    assert!((result.commission.to_f64().unwrap() - 5.0).abs() < 0.000001);
    let active = manager.get_active_orders().await;
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].filled_quantity, dec!(0.5));
    assert_eq!(active[0].commission, result.commission);
}

struct ZeroFeeExchange;

#[async_trait::async_trait]
impl LiveExchange for ZeroFeeExchange {
    async fn execute_order(&self, _order: &Order) -> quant_common::Result<String> {
        Ok("456".to_string())
    }

    async fn cancel_order(&self, _symbol: &str, _order_id: &str) -> quant_common::Result<()> {
        panic!("cancellation is not part of this execution")
    }

    async fn get_order_details(
        &self,
        _symbol: &str,
        _order_id: &str,
    ) -> quant_common::Result<crate::live_exchange::OrderDetails> {
        Ok(crate::live_exchange::OrderDetails {
            avg_price: dec!(10000),
            filled_quantity: dec!(2),
            status: OrderStatus::Filled,
            fee: Some(Decimal::ZERO),
        })
    }
}

#[tokio::test]
async fn test_exchange_reported_zero_fee_is_not_replaced_by_estimate() {
    let manager = Arc::new(OrderManager::new());
    let strategy = BinanceExecutionStrategy::new(
        manager.clone(),
        config(false, 0, 0.0, 0.001),
        Some(Arc::new(ZeroFeeExchange)),
    );
    let id = manager
        .submit_order(sample_order(OrderSide::Buy, Some(dec!(10000))))
        .await
        .unwrap();
    let result = strategy
        .execute(
            manager.get_order(id).await.unwrap(),
            &sample_market(dec!(10000)),
        )
        .await
        .unwrap();
    assert_eq!(result.commission, Decimal::ZERO);
    assert_eq!(
        manager.get_order(id).await.unwrap().commission,
        Decimal::ZERO
    );
}

#[tokio::test]
async fn test_live_strategy_cannot_submit_through_paper_calculation() {
    let mock = MockBinanceClient::new();
    let manager = Arc::new(OrderManager::new());
    let engine = binance_engine(manager, mock);
    assert!(matches!(
        engine
            .calculate_paper_fill(
                sample_order(OrderSide::Buy, Some(dec!(10000))),
                &sample_market(dec!(10000))
            )
            .await,
        Err(Error::Trading(_))
    ));
}

#[tokio::test]
async fn test_paper_rejects_slippage_that_produces_nonpositive_fill_price() {
    let manager = Arc::new(OrderManager::new());
    let engine = ExecutionEngine::new_binance(manager, config(true, 0, 1.0, 0.0), None);
    assert!(matches!(
        engine
            .calculate_paper_fill(
                sample_order(OrderSide::Sell, None),
                &sample_market(dec!(100))
            )
            .await,
        Err(Error::Validation(_))
    ));
}
