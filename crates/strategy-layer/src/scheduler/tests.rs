//! StrategyScheduler 单元测试。

use super::*;
use crate::pipeline::{OrderExecStep, PipelineExecutor};
use crate::signals::Signal;
use crate::strategy::{MeanReversionStrategy, StrategyContext};
use crate::traits::{OrderExecError, OrderExecutor};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_common::types::{
    MarketData, Order, OrderSide, OrderStatus, OrderType, StrategyParams, StrategyType,
};
use std::sync::{Arc, Mutex};
use tokio::time::Duration;

/// Mock provider that returns no historical data. Enough to satisfy the
/// scheduler's "trading ready" precondition for lifecycle tests.
struct MockProvider;

#[async_trait]
impl MarketDataProvider for MockProvider {
    async fn get_historical_data(
        &self,
        _symbol: &str,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<MarketData>, String> {
        Ok(Vec::new())
    }
}

fn make_scheduler() -> StrategyScheduler {
    let config = SchedulerConfig {
        enabled: true,
        ..Default::default()
    };
    let scheduler = StrategyScheduler::new(config);
    scheduler.set_pipeline(Arc::new(PipelineExecutor::new()));
    scheduler.set_market_data_provider(Arc::new(MockProvider));
    scheduler
}

async fn make_dummy_strategy() -> Box<dyn Strategy> {
    let mut s = MeanReversionStrategy::new();
    let params = StrategyParams::builder(
        "test_scheduler".to_string(),
        "Scheduler Test".to_string(),
        StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(rust_decimal::Decimal::new(100000, 0))
    .max_daily_loss(rust_decimal::Decimal::new(5000, 0))
    .build();
    // 忽略 initialize 错误
    let _ = s.initialize(params).await;
    Box::new(s)
}

#[tokio::test]
async fn test_scheduler_starts_empty() {
    let scheduler = make_scheduler();
    assert_eq!(scheduler.running_count().await, 0);
    assert!(scheduler.list_running().await.is_empty());
}

#[tokio::test]
async fn test_start_and_stop_strategy() {
    let scheduler = make_scheduler();
    scheduler
        .start_strategy(
            "test_001".to_string(),
            "Test Strategy".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await
        .unwrap();
    assert_eq!(scheduler.running_count().await, 1);

    scheduler.stop_strategy("test_001").await.unwrap();
    assert_eq!(scheduler.running_count().await, 0);
}

#[tokio::test]
async fn test_start_twice_returns_error() {
    let scheduler = make_scheduler();
    scheduler
        .start_strategy(
            "dup".to_string(),
            "Dup Strategy".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await
        .unwrap();

    let result = scheduler
        .start_strategy(
            "dup".to_string(),
            "Dup Strategy".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SchedulerError::AlreadyRunning(_)
    ));
}

#[tokio::test]
async fn test_stop_nonexistent_returns_error() {
    let scheduler = make_scheduler();
    let result = scheduler.stop_strategy("nonexistent").await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), SchedulerError::NotFound(_)));
}

#[tokio::test]
async fn test_shutdown_all() {
    let scheduler = make_scheduler();
    scheduler
        .start_strategy(
            "s1".to_string(),
            "Strategy 1".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await
        .unwrap();
    scheduler
        .start_strategy(
            "s2".to_string(),
            "Strategy 2".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await
        .unwrap();
    assert_eq!(scheduler.running_count().await, 2);

    scheduler.shutdown_all().await;
    assert_eq!(scheduler.running_count().await, 0);
}

#[tokio::test]
async fn test_circuit_breaker_initialized() {
    let scheduler = make_scheduler();
    scheduler
        .start_strategy(
            "cb_test".to_string(),
            "CB Test".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await
        .unwrap();

    let status = scheduler.circuit_breaker_status("cb_test").await;
    assert!(status.is_some());
    assert!(!status.unwrap().is_tripped());
}

#[tokio::test]
async fn test_start_without_pipeline_fails_closed() {
    // A scheduler with no pipeline / market-data provider must refuse to
    // start a strategy (fail-closed) instead of appearing "Running" while idle.
    let scheduler = StrategyScheduler::new(SchedulerConfig::default());
    let result = scheduler
        .start_strategy(
            "nc".to_string(),
            "Not Configured".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SchedulerError::NotConfigured(_)
    ));
}

#[tokio::test]
async fn test_start_with_scheduler_disabled_fails_closed() {
    // Even when pipeline + provider are wired, a scheduler whose
    // `SchedulerConfig::enabled` is false must refuse to run (fail-closed).
    // `enabled` defaults to true now, so set it explicitly to exercise the
    // disabled fail-closed path.
    let config = SchedulerConfig {
        enabled: false,
        ..Default::default()
    };
    let scheduler = StrategyScheduler::new(config);
    scheduler.set_pipeline(Arc::new(PipelineExecutor::new()));
    scheduler.set_market_data_provider(Arc::new(MockProvider));
    let result = scheduler
        .start_strategy(
            "nc_disabled".to_string(),
            "Disabled".to_string(),
            make_dummy_strategy().await,
            3600,
        )
        .await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SchedulerError::NotConfigured(_)
    ));
}

/// A strategy that always emits the same order regardless of market data.
/// Lets us assert the scheduler loop: fetch data → generate signals → run each
/// order through the pipeline.
struct EmitOrderStrategy {
    params: StrategyParams,
    order: Order,
}

#[async_trait]
impl Strategy for EmitOrderStrategy {
    async fn initialize(&mut self, _params: StrategyParams) -> quant_common::Result<()> {
        Ok(())
    }

    async fn generate_signals(&self, _ctx: &StrategyContext) -> quant_common::Result<Vec<Order>> {
        Ok(vec![self.order.clone()])
    }

    fn name(&self) -> &str {
        "emit-order"
    }

    fn params(&self) -> &StrategyParams {
        &self.params
    }

    fn params_mut(&mut self) -> &mut StrategyParams {
        &mut self.params
    }
}

/// Records the signals that pass through the pipeline's execution step, acting
/// as the minimal "paper execution" observer in this test.
#[derive(Clone)]
struct RecordingExecutor {
    recorded: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl OrderExecutor for RecordingExecutor {
    async fn execute(&self, signal: &Signal) -> Result<String, OrderExecError> {
        let mut guard = self.recorded.lock().unwrap();
        guard.push(format!(
            "{}:{:?}:{:?}",
            signal.symbol, signal.signal_type, signal.price
        ));
        Ok("rec-1".to_string())
    }
}

fn emit_buy_order() -> Order {
    Order {
        order_id: 0,
        strategy_id: "chain".to_string(),
        symbol: "BTC-USDT".to_string(),
        order_type: OrderType::Limit,
        side: OrderSide::Buy,
        price: Some(rust_decimal::Decimal::new(50000, 0)),
        quantity: rust_decimal::Decimal::new(1, 0),
        filled_quantity: rust_decimal::Decimal::ZERO,
        status: OrderStatus::Pending,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        commission: rust_decimal::Decimal::ZERO,
        slippage: rust_decimal::Decimal::ZERO,
    }
}

#[tokio::test]
async fn test_scheduler_routes_generated_signal_to_pipeline() {
    // Full chain: mock provider supplies market data → strategy generates an
    // order → scheduler routes it through the pipeline → OrderExecStep.
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let pipeline = PipelineExecutor::new();
    pipeline
        .add_step(Box::new(OrderExecStep::new(Box::new(RecordingExecutor {
            recorded: recorded.clone(),
        }))))
        .await;

    let config = SchedulerConfig {
        enabled: true,
        ..Default::default()
    };
    let scheduler = StrategyScheduler::new(config);
    scheduler.set_pipeline(Arc::new(pipeline));
    scheduler.set_market_data_provider(Arc::new(MockProvider));

    let strategy = EmitOrderStrategy {
        params: StrategyParams::builder(
            "chain".to_string(),
            "Chain Strategy".to_string(),
            StrategyType::MeanReversion,
        )
        .params(serde_json::json!({}))
        .max_position(rust_decimal::Decimal::new(100000, 0))
        .max_daily_loss(rust_decimal::Decimal::new(5000, 0))
        .symbols(vec!["BTC-USDT".to_string()])
        .build(),
        order: emit_buy_order(),
    };

    scheduler
        .start_strategy(
            "chain".to_string(),
            "Chain".to_string(),
            Box::new(strategy),
            1,
        )
        .await
        .unwrap();

    // Wait past one interval so the scheduler generates a signal and runs it
    // through the pipeline (interval=1s; first execution ~1s).
    tokio::time::sleep(Duration::from_millis(1600)).await;

    scheduler.stop_strategy("chain").await.unwrap();

    let rec = recorded.lock().unwrap();
    assert!(
        !rec.is_empty(),
        "scheduler must route a generated order through the pipeline"
    );
    assert!(
        rec.iter()
            .any(|s| s.contains("BTC-USDT") && s.contains("Buy")),
        "expected a BTC-USDT buy signal to reach the pipeline, got {:?}",
        *rec
    );
}
