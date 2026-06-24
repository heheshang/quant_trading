use crate::okx_executor::OkxExecutor;
use crate::order_manager::OrderManager;
use quant_common::config::TradingConfig;
use quant_common::types::{MarketData, Order, OrderStatus};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// 执行引擎
pub struct ExecutionEngine {
    order_manager: Arc<OrderManager>,
    config: TradingConfig,
    okx_executor: Option<Arc<OkxExecutor>>,
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
        }
    }

    /// 执行订单
    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side, paper = %self.config.enable_paper_trading))]
    pub async fn execute_order(&self, order: Order, market_data: &MarketData) -> Result<()> {
        info!("Executing order: {:?}", order.order_id);

        // 模拟订单执行（实际需要对接交易所API）
        if self.config.enable_paper_trading {
            self.simulate_execution(order, market_data).await
        } else {
            self.real_execution(order, market_data).await
        }
    }

    /// 模拟盘执行
    async fn simulate_execution(&self, mut order: Order, market_data: &MarketData) -> Result<()> {
        // 计算成交价格（考虑滑点）
        let execution_price = self.calculate_execution_price(&order, market_data)?;

        // 计算手续费
        let total_value = execution_price * order.quantity;
        let commission = total_value
            * Decimal::from_f64_retain(self.config.default_commission_rate)
                .unwrap_or(Decimal::ZERO);

        order.filled_quantity = order.quantity;
        order.commission = commission;
        order.status = OrderStatus::Filled;
        order.updated_at = chrono::Utc::now();

        self.order_manager
            .update_order_status(order.order_id, OrderStatus::Filled)
            .await?;

        info!(
            "Order {} filled at price {}",
            order.order_id, execution_price
        );
        Ok(())
    }

    /// 实盘执行
    async fn real_execution(&self, order: Order, _market_data: &MarketData) -> Result<()> {
        match &self.okx_executor {
            Some(executor) => {
                let order_id = executor.execute_order(&order).await.map_err(|e| {
                    error!(order_id = %order.order_id, error = %e, "OKX real execution failed");
                    Error::Internal(format!("OKX execution failed: {}", e))
                })?;

                info!(
                    order_id = %order.order_id,
                    okx_order_id = %order_id,
                    "Order executed on OKX"
                );

                self.order_manager
                    .update_order_status(order.order_id, OrderStatus::Filled)
                    .await?;

                Ok(())
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
