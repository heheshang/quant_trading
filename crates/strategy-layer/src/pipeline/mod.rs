//! Pipeline — 策略订单处理流水线
//!
//! Pipeline 是一个步骤链，负责对策略生成的订单进行逐级处理：
//! 1. RiskCheck — 风控检查
//! 2. OrderExec — 订单执行

mod steps;

pub use steps::*;

use quant_common::types::Order;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::error;

/// Pipeline 步骤执行器
#[async_trait::async_trait]
pub trait PipelineStep: Send + Sync {
    /// 执行当前步骤
    ///
    /// # Errors
    ///
    /// 返回错误时中断流水线后续步骤。
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError>;
}

/// Pipeline 上下文，在步骤之间传递数据
#[derive(Clone, Debug, serde::Serialize)]
pub struct PipelineContext {
    /// 待处理订单
    pub order: Order,
    /// 风控是否通过
    pub risk_approved: bool,
    /// 风控拒绝原因
    pub risk_reason: Option<String>,
    /// 执行状态
    pub execution_status: ExecutionStatus,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum ExecutionStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed(String),
}

impl PipelineContext {
    /// 从订单创建初始上下文
    #[must_use]
    pub fn new(order: Order) -> Self {
        Self {
            risk_approved: false,
            risk_reason: None,
            execution_status: ExecutionStatus::Pending,
            order,
        }
    }
}

/// Pipeline 执行器，管理步骤链
pub struct PipelineExecutor {
    steps: Arc<RwLock<Vec<Box<dyn PipelineStep>>>>,
}

impl PipelineExecutor {
    /// 创建空流水线
    #[must_use]
    pub fn new() -> Self {
        Self {
            steps: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 添加步骤到流水线末尾
    pub async fn add_step(&self, step: Box<dyn PipelineStep>) {
        let mut steps = self.steps.write().await;
        steps.push(step);
    }

    /// 执行订单通过整个流水线
    ///
    /// 按序执行每个步骤，任一步骤返回错误则中断。
    pub async fn execute(&self, order: Order) -> Result<PipelineContext, PipelineError> {
        let mut ctx = PipelineContext::new(order);

        let steps = self.steps.read().await;

        for step in steps.iter() {
            if let Err(e) = step.execute(&mut ctx).await {
                error!(error = %e, "Pipeline step failed");
                ctx.execution_status = ExecutionStatus::Failed(e.to_string());
                return Err(e);
            }
        }

        Ok(ctx)
    }

    /// 获取当前步骤数量
    #[must_use]
    pub async fn step_count(&self) -> usize {
        self.steps.read().await.len()
    }

    /// 清空流水线
    pub async fn clear(&self) {
        self.steps.write().await.clear();
    }
}

impl Default for PipelineExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Pipeline 错误
#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum PipelineError {
    #[error("Risk check rejected: {0}")]
    RiskRejected(String),

    #[error("Order execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Pipeline error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::types::{OrderSide, OrderType};

    struct MockPassStep;

    #[async_trait::async_trait]
    impl PipelineStep for MockPassStep {
        async fn execute(&self, _ctx: &mut PipelineContext) -> Result<(), PipelineError> {
            Ok(())
        }
    }

    struct MockFailStep;

    #[async_trait::async_trait]
    impl PipelineStep for MockFailStep {
        async fn execute(&self, _ctx: &mut PipelineContext) -> Result<(), PipelineError> {
            Err(PipelineError::Other("mock failure".to_string()))
        }
    }

    fn sample_order() -> Order {
        Order::new(
            "strat-001".to_string(),
            "BTC/USDT".to_string(),
            OrderType::Limit,
            OrderSide::Buy,
            Some(rust_decimal::Decimal::new(50000, 0)),
            rust_decimal::Decimal::new(1, 0),
        )
    }

    #[tokio::test]
    async fn test_empty_pipeline_passthrough() {
        let pipeline = PipelineExecutor::new();
        let order = sample_order();
        let result = pipeline.execute(order).await;
        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.order.strategy_id, "strat-001");
        assert_eq!(ctx.execution_status, ExecutionStatus::Pending);
    }

    #[tokio::test]
    async fn test_pipeline_with_passing_steps() {
        let pipeline = PipelineExecutor::new();
        pipeline.add_step(Box::new(MockPassStep)).await;
        pipeline.add_step(Box::new(MockPassStep)).await;

        let order = sample_order();
        let result = pipeline.execute(order).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pipeline_stops_on_failure() {
        let pipeline = PipelineExecutor::new();
        pipeline.add_step(Box::new(MockPassStep)).await;
        pipeline.add_step(Box::new(MockFailStep)).await;
        pipeline.add_step(Box::new(MockPassStep)).await;

        let order = sample_order();
        let result = pipeline.execute(order).await;
        assert!(result.is_err());
        assert_eq!(pipeline.step_count().await, 3);
    }

    #[tokio::test]
    async fn test_step_count_starts_zero() {
        let pipeline = PipelineExecutor::new();
        assert_eq!(pipeline.step_count().await, 0);
    }

    #[tokio::test]
    async fn test_clear_removes_all_steps() {
        let pipeline = PipelineExecutor::new();
        pipeline.add_step(Box::new(MockPassStep)).await;
        pipeline.add_step(Box::new(MockPassStep)).await;
        assert_eq!(pipeline.step_count().await, 2);
        pipeline.clear().await;
        assert_eq!(pipeline.step_count().await, 0);
    }
}
