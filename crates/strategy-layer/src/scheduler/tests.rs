//! StrategyScheduler 单元测试。

use super::*;
use crate::strategy::MeanReversionStrategy;
use quant_common::types::StrategyParams;
use quant_common::types::StrategyType;

fn make_scheduler() -> StrategyScheduler {
    StrategyScheduler::new(SchedulerConfig::default())
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
