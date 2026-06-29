//! Snapshot tests for pipeline behaviour (AC-D).
//!
//! These tests verify that the DIP refactoring does not change the pipeline's
//! observable behaviour: step ordering, context mutation, and error propagation
//! are identical to the pre-refactoring code.

use async_trait::async_trait;
use quant_common::types::{Order, OrderSide, OrderType};
use strategy_engine::pipeline::{OrderExecStep, PipelineExecutor, RiskCheckStep};
use strategy_engine::signals::Signal;
use strategy_engine::traits::{OrderExecError, OrderExecutor, RiskCheckError, RiskChecker};

// ---------------------------------------------------------------------------
// Mock implementations
// ---------------------------------------------------------------------------

struct AlwaysPassRiskChecker;

#[async_trait]
impl RiskChecker for AlwaysPassRiskChecker {
    async fn check(&self, _signal: &Signal) -> Result<(), RiskCheckError> {
        Ok(())
    }
}

struct AlwaysFailRiskChecker;

#[async_trait]
impl RiskChecker for AlwaysFailRiskChecker {
    async fn check(&self, _signal: &Signal) -> Result<(), RiskCheckError> {
        Err(RiskCheckError::Rejected("test rejection".to_string()))
    }
}

struct AlwaysPassExecutor;

#[async_trait]
impl OrderExecutor for AlwaysPassExecutor {
    async fn execute(&self, _signal: &Signal) -> Result<String, OrderExecError> {
        Ok("exec-pass-001".to_string())
    }
}

struct AlwaysFailExecutor;

#[async_trait]
impl OrderExecutor for AlwaysFailExecutor {
    async fn execute(&self, _signal: &Signal) -> Result<String, OrderExecError> {
        Err(OrderExecError::Rejected(
            "test execution rejection".to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn buy_order() -> Order {
    Order::new(
        "strat-test".to_string(),
        "BTC/USDT".to_string(),
        OrderType::Limit,
        OrderSide::Buy,
        Some(rust_decimal::Decimal::new(50000, 0)),
        rust_decimal::Decimal::new(1, 0),
    )
}

fn sell_order() -> Order {
    Order::new(
        "strat-test".to_string(),
        "ETH/USDT".to_string(),
        OrderType::Limit,
        OrderSide::Sell,
        Some(rust_decimal::Decimal::new(3000, 0)),
        rust_decimal::Decimal::new(2, 0),
    )
}

// ---------------------------------------------------------------------------
// AC-D: Snapshot tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn snapshot_pipeline_passthrough_no_engines() {
    let pipeline = PipelineExecutor::new();
    pipeline
        .add_step(Box::new(RiskCheckStep::passthrough()))
        .await;
    pipeline
        .add_step(Box::new(OrderExecStep::passthrough()))
        .await;

    let result = pipeline.execute(buy_order()).await.unwrap();

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(result, {
            ".order.order_id" => "[order_id]",
            ".order.created_at" => "[created_at]",
            ".order.updated_at" => "[updated_at]",
        });
    });
}

#[tokio::test]
async fn snapshot_pipeline_risk_pass_exec_success() {
    let pipeline = PipelineExecutor::new();
    pipeline
        .add_step(Box::new(RiskCheckStep::new(Box::new(
            AlwaysPassRiskChecker,
        ))))
        .await;
    pipeline
        .add_step(Box::new(OrderExecStep::new(Box::new(
            AlwaysPassExecutor,
        ))))
        .await;

    let result = pipeline.execute(buy_order()).await.unwrap();

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(result, {
            ".order.order_id" => "[order_id]",
            ".order.created_at" => "[created_at]",
            ".order.updated_at" => "[updated_at]",
        });
    });
}

#[tokio::test]
async fn snapshot_pipeline_risk_rejected_stops_pipeline() {
    let pipeline = PipelineExecutor::new();
    pipeline
        .add_step(Box::new(RiskCheckStep::new(Box::new(
            AlwaysFailRiskChecker,
        ))))
        .await;
    pipeline
        .add_step(Box::new(OrderExecStep::passthrough()))
        .await;

    let result = pipeline.execute(buy_order()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(err);
    });
}

#[tokio::test]
async fn snapshot_pipeline_exec_fails_after_risk_pass() {
    let pipeline = PipelineExecutor::new();
    pipeline
        .add_step(Box::new(RiskCheckStep::new(Box::new(
            AlwaysPassRiskChecker,
        ))))
        .await;
    pipeline
        .add_step(Box::new(OrderExecStep::new(Box::new(
            AlwaysFailExecutor,
        ))))
        .await;

    let result = pipeline.execute(sell_order()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(err);
    });
}

#[tokio::test]
async fn snapshot_pipeline_sell_order_exec_success() {
    let pipeline = PipelineExecutor::new();
    pipeline
        .add_step(Box::new(RiskCheckStep::new(Box::new(
            AlwaysPassRiskChecker,
        ))))
        .await;
    pipeline
        .add_step(Box::new(OrderExecStep::new(Box::new(
            AlwaysPassExecutor,
        ))))
        .await;

    let result = pipeline.execute(sell_order()).await.unwrap();

    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(result, {
            ".order.order_id" => "[order_id]",
            ".order.created_at" => "[created_at]",
            ".order.updated_at" => "[updated_at]",
        });
    });
}
