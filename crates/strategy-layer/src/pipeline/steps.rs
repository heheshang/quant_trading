use super::{ExecutionStatus, PipelineContext, PipelineError, PipelineStep};
use quant_common::types::{Account, OrderSide, Position, MarketData};
use rust_decimal::Decimal;
use chrono::Utc;
use tracing::info;

pub struct RiskCheckStep;

#[async_trait::async_trait]
impl PipelineStep for RiskCheckStep {
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        match ctx.order.side {
            OrderSide::Buy | OrderSide::Sell => {
                // 使用真实风控检查器
                if let Some(risk_checker) = &ctx.risk_checker {
                    // 创建必要的账户和持仓数据
                    let account = Account {
                        account_id: 0,
                        total_assets: Decimal::ZERO,
                        available_cash: Decimal::ZERO,
                        frozen_cash: Decimal::ZERO,
                        market_value: Decimal::ZERO,
                        total_pnl: Decimal::ZERO,
                        daily_pnl: Decimal::ZERO,
                        margin: Decimal::ZERO,
                        margin_ratio: Decimal::ZERO,
                        updated_at: Utc::now(),
                    };
                    let positions: Vec<Position> = Vec::new();
                    
                    // 执行风控检查
                    match risk_checker.check_order(&ctx.order, &account, &positions) {
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
                    // 向后兼容：如果没有风控检查器，直接批准
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
    }
}

pub struct OrderExecStep;

#[async_trait::async_trait]
impl PipelineStep for OrderExecStep {
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<(), PipelineError> {
        // 使用真实执行引擎
                if let Some(execution_engine) = &ctx.execution_engine {
            // 创建市场数据
            let market_data = MarketData {
                symbol: ctx.order.symbol.clone(),
                timestamp: Utc::now(),
                open: ctx.order.price.unwrap_or(Decimal::ZERO),
                high: ctx.order.price.unwrap_or(Decimal::ZERO),
                low: ctx.order.price.unwrap_or(Decimal::ZERO),
                close: ctx.order.price.unwrap_or(Decimal::ZERO),
                volume: Decimal::ZERO,
                turnover: Decimal::ZERO,
                open_interest: None,
                bid_prices: Vec::new(),
                bid_volumes: Vec::new(),
                ask_prices: Vec::new(),
                ask_volumes: Vec::new(),
            };
            
            // 执行订单
            match execution_engine.execute_order(ctx.order.clone(), &market_data).await {
                Ok(result) => {
                    ctx.execution_status = ExecutionStatus::Confirmed;
                    info!(
                        strategy_id = %ctx.order.strategy_id,
                        symbol = %ctx.order.symbol,
                        order_id = %result.order_id,
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
            // 向后兼容：如果没有执行引擎，直接标记为已提交
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
