use crate::okx_executor::OkxExecutor;
use crate::order_manager::OrderManager;
use chrono::{DateTime, Utc};
use quant_common::config::TradingConfig;
use quant_common::types::{MarketData, Order, OrderStatus};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// 执行结果
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub order_id: i64,
    pub strategy_id: String,
    pub symbol: String,
    pub filled_quantity: Decimal,
    pub avg_price: Decimal,
    pub commission: Decimal,
    pub status: OrderStatus,
    pub executed_at: DateTime<Utc>,
}

/// 策略执行回调
#[async_trait::async_trait]
pub trait ExecutionCallback: Send + Sync {
    async fn on_order_executed(&self, result: &ExecutionResult);
}

/// 执行引擎
pub struct ExecutionEngine {
    order_manager: Arc<OrderManager>,
    config: TradingConfig,
    okx_executor: Option<Arc<OkxExecutor>>,
    callbacks: Vec<Box<dyn ExecutionCallback>>,
}

impl ExecutionEngine {
    pub fn new(
        order_manager: Arc<OrderManager>,
        config: TradingConfig,
        okx_executor: Option<Arc<OkxExecutor>>,
    ) -> Self {
        Self {
            order_manager,
            config,
            okx_executor,
            callbacks: Vec::new(),
        }
    }

    /// 注册执行回调
    pub fn register_callback(&mut self, callback: Box<dyn ExecutionCallback>) {
        self.callbacks.push(callback);
    }

    /// 执行订单
    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side, paper = %self.config.enable_paper_trading))]
    pub async fn execute_order(
        &self,
        order: Order,
        market_data: &MarketData,
    ) -> Result<ExecutionResult> {
        info!("Executing order: {:?}", order.order_id);

        let result = if self.config.enable_paper_trading {
            self.simulate_execution(order, market_data).await
        } else {
            self.real_execution(order, market_data).await
        };

        // 通知回调
        if let Ok(ref r) = result {
            for cb in &self.callbacks {
                cb.on_order_executed(r).await;
            }
        }

        result
    }

    /// 模拟盘执行
    async fn simulate_execution(
        &self,
        mut order: Order,
        market_data: &MarketData,
    ) -> Result<ExecutionResult> {
        let delay_ms = self.config.simulation_delay_ms;
        if delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }

        let execution_price = self.calculate_execution_price(&order, market_data)?;

        let total_value = execution_price * order.quantity;
        let commission = total_value
            * Decimal::from_f64_retain(self.config.default_commission_rate)
                .unwrap_or(Decimal::ZERO);

        order.filled_quantity = order.quantity;
        order.commission = commission;
        order.status = OrderStatus::Filled;
        order.updated_at = Utc::now();

        self.order_manager
            .update_order_status(order.order_id, OrderStatus::Filled)
            .await?;

        let result = ExecutionResult {
            order_id: order.order_id,
            strategy_id: order.strategy_id.clone(),
            symbol: order.symbol.clone(),
            filled_quantity: order.filled_quantity,
            avg_price: execution_price,
            commission,
            status: OrderStatus::Filled,
            executed_at: Utc::now(),
        };

        info!(
            "Order {} filled at price {}",
            order.order_id, execution_price
        );
        Ok(result)
    }

    /// 实盘执行
    async fn real_execution(
        &self,
        order: Order,
        _market_data: &MarketData,
    ) -> Result<ExecutionResult> {
        match &self.okx_executor {
            Some(executor) => {
                let okx_order_id = executor.execute_order(&order).await.map_err(|e| {
                    error!(order_id = %order.order_id, error = %e, "OKX real execution failed");
                    Error::Internal(format!("OKX execution failed: {}", e))
                })?;

                info!(
                    order_id = %order.order_id,
                    okx_order_id = %okx_order_id,
                    "Order executed on OKX"
                );

                self.order_manager
                    .update_order_status(order.order_id, OrderStatus::Filled)
                    .await?;

                Ok(ExecutionResult {
                    order_id: order.order_id,
                    strategy_id: order.strategy_id,
                    symbol: order.symbol,
                    filled_quantity: order.quantity,
                    avg_price: order.price.unwrap_or(Decimal::ZERO),
                    commission: Decimal::ZERO,
                    status: OrderStatus::Filled,
                    executed_at: Utc::now(),
                })
            }
            None => {
                error!("No OKX executor configured for real execution");
                Err(Error::Internal("No OKX executor configured".to_string()))
            }
        }
    }

    /// 计算执行价格（含滑点）
    fn calculate_execution_price(
        &self,
        order: &Order,
        market_data: &MarketData,
    ) -> Result<Decimal> {
        let base_price = order.price.unwrap_or(market_data.close);
        let slippage_factor =
            Decimal::from_f64_retain(self.config.default_slippage).unwrap_or(Decimal::ZERO);

        let slippage_amount = base_price * slippage_factor;

        let execution_price = match order.side {
            quant_common::types::OrderSide::Buy => base_price + slippage_amount,
            quant_common::types::OrderSide::Sell => base_price - slippage_amount,
        };

        Ok(execution_price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCallback;

    #[async_trait::async_trait]
    impl ExecutionCallback for TestCallback {
        async fn on_order_executed(&self, _result: &ExecutionResult) {}
    }

    #[test]
    fn test_execution_result_creation() {
        let result = ExecutionResult {
            order_id: 1,
            strategy_id: "test".to_string(),
            symbol: "BTC/USDT".to_string(),
            filled_quantity: Decimal::new(1, 0),
            avg_price: Decimal::new(50000, 0),
            commission: Decimal::ZERO,
            status: OrderStatus::Filled,
            executed_at: Utc::now(),
        };
        assert_eq!(result.order_id, 1);
        assert_eq!(result.symbol, "BTC/USDT");
    }

    #[test]
    fn test_callback_registration() {
        let config = TradingConfig {
            enable_paper_trading: true,
            max_orders_per_second: 10,
            default_commission_rate: 0.001,
            default_slippage: 0.0005,
            order_timeout_seconds: 30,
            simulation_delay_ms: 100,
        };
        let mut engine = ExecutionEngine::new(
            Arc::new(OrderManager::new()),
            config,
            None,
        );
        engine.register_callback(Box::new(TestCallback));
        assert_eq!(engine.callbacks.len(), 1);
    }
}
