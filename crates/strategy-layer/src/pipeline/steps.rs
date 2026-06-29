use super::{ExecutionStatus, PipelineContext, PipelineError, PipelineStep};
use crate::signals::{Signal, SignalSource, SignalType};
use crate::traits::{OrderExecutor, RiskChecker};
use chrono::Utc;
use quant_common::types::{Order, OrderSide};
use tracing::info;

/// Convert an Order into a Signal for use with trait-based steps.
fn order_to_signal(order: &Order) -> Signal {
    Signal {
        signal_type: match order.side {
            OrderSide::Buy => SignalType::Buy,
            OrderSide::Sell => SignalType::Sell,
        },
        symbol: order.symbol.clone(),
        strength: 1.0,
        price: order.price,
        quantity: Some(order.quantity),
        id: format!("sig-from-order-{}", order.order_id),
        strategy_id: order.strategy_id.clone(),
        source: SignalSource::Strategy,
        generated_at: Utc::now(),
        metadata: serde_json::Value::Null,
    }
}

/// Risk check pipeline step — delegates to an injected [`RiskChecker`].
pub struct RiskCheckStep {
    checker: Option<Box<dyn RiskChecker>>,
}

impl RiskCheckStep {
    /// Create a step that delegates risk decisions to `checker`.
    pub fn new(checker: Box<dyn RiskChecker>) -> Self {
        Self {
            checker: Some(checker),
        }
    }

    /// Create a passthrough step that approves every order (no checker).
    pub fn passthrough() -> Self {
        Self { checker: None }
    }
}

#[async_trait::async_trait]
impl PipelineStep for RiskCheckStep {
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if let Some(checker) = &self.checker {
            let signal = order_to_signal(&ctx.order);
            match checker.check(&signal).await {
                Ok(()) => {
                    ctx.risk_approved = true;
                    info!(
                        strategy_id = %ctx.order.strategy_id,
                        symbol = %ctx.order.symbol,
                        side = ?ctx.order.side,
                        "Risk check passed"
                    );
                    Ok(())
                }
                Err(e) => {
                    ctx.risk_approved = false;
                    ctx.risk_reason = Some(e.to_string());
                    info!(
                        strategy_id = %ctx.order.strategy_id,
                        symbol = %ctx.order.symbol,
                        side = ?ctx.order.side,
                        risk_reason = %e,
                        "Risk check failed"
                    );
                    Err(PipelineError::RiskRejected(e.to_string()))
                }
            }
        } else {
            // Passthrough: approve without a checker
            ctx.risk_approved = true;
            info!(
                strategy_id = %ctx.order.strategy_id,
                symbol = %ctx.order.symbol,
                side = ?ctx.order.side,
                "Risk check passed (no checker)"
            );
            Ok(())
        }
    }
}

/// Order execution pipeline step — delegates to an injected [`OrderExecutor`].
pub struct OrderExecStep {
    executor: Option<Box<dyn OrderExecutor>>,
}

impl OrderExecStep {
    /// Create a step that delegates order execution to `executor`.
    pub fn new(executor: Box<dyn OrderExecutor>) -> Self {
        Self {
            executor: Some(executor),
        }
    }

    /// Create a passthrough step that marks orders as submitted (no executor).
    pub fn passthrough() -> Self {
        Self { executor: None }
    }
}

#[async_trait::async_trait]
impl PipelineStep for OrderExecStep {
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        if let Some(executor) = &self.executor {
            let signal = order_to_signal(&ctx.order);
            match executor.execute(&signal).await {
                Ok(result) => {
                    ctx.execution_status = ExecutionStatus::Confirmed;
                    info!(
                        strategy_id = %ctx.order.strategy_id,
                        symbol = %ctx.order.symbol,
                        order_id = %result,
                        "Order executed successfully"
                    );
                    Ok(())
                }
                Err(e) => {
                    ctx.execution_status = ExecutionStatus::Failed(e.to_string());
                    info!(
                        strategy_id = %ctx.order.strategy_id,
                        symbol = %ctx.order.symbol,
                        error = %e,
                        "Order execution failed"
                    );
                    Err(PipelineError::ExecutionFailed(e.to_string()))
                }
            }
        } else {
            // Passthrough: mark as submitted
            ctx.execution_status = ExecutionStatus::Submitted;
            info!(
                strategy_id = %ctx.order.strategy_id,
                symbol = %ctx.order.symbol,
                "Order submitted for execution (no engine)"
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::types::{OrderSide, OrderType};

    fn buy_order() -> Order {
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
    async fn test_risk_check_passthrough_approves_buy_order() {
        let step = RiskCheckStep::passthrough();
        let mut ctx = PipelineContext::new(buy_order());
        assert!(step.execute(&mut ctx).await.is_ok());
        assert!(ctx.risk_approved);
    }

    #[tokio::test]
    async fn test_order_exec_passthrough_marks_submitted() {
        let step = OrderExecStep::passthrough();
        let mut ctx = PipelineContext::new(buy_order());
        assert!(step.execute(&mut ctx).await.is_ok());
        assert_eq!(ctx.execution_status, ExecutionStatus::Submitted);
    }
}
