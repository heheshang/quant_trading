use super::{ExecutionStatus, PipelineContext, PipelineError, PipelineStep};
use quant_common::types::OrderSide;
use tracing::info;

pub struct RiskCheckStep;

#[async_trait::async_trait]
impl PipelineStep for RiskCheckStep {
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        match ctx.order.side {
            OrderSide::Buy | OrderSide::Sell => {
                ctx.risk_approved = true;
                info!(
                    strategy_id = %ctx.order.strategy_id,
                    symbol = %ctx.order.symbol,
                    side = ?ctx.order.side,
                    "Risk check passed"
                );
                Ok(())
            }
        }
    }
}

pub struct OrderExecStep;

#[async_trait::async_trait]
impl PipelineStep for OrderExecStep {
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        ctx.execution_status = ExecutionStatus::Submitted;
        info!(
            strategy_id = %ctx.order.strategy_id,
            symbol = %ctx.order.symbol,
            "Order submitted for execution"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quant_common::types::{Order, OrderSide, OrderType};

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
    async fn test_risk_check_approves_buy_order() {
        let step = RiskCheckStep;
        let mut ctx = PipelineContext::new(buy_order());
        assert!(step.execute(&mut ctx).await.is_ok());
        assert!(ctx.risk_approved);
    }

    #[tokio::test]
    async fn test_order_exec_marks_submitted() {
        let step = OrderExecStep;
        let mut ctx = PipelineContext::new(buy_order());
        assert!(step.execute(&mut ctx).await.is_ok());
        assert_eq!(ctx.execution_status, ExecutionStatus::Submitted);
    }
}
