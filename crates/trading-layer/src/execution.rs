use quant_common::{Error, Result};
use quant_common::types::{Order, OrderStatus, MarketData};
use quant_common::config::TradingConfig;
use crate::order_manager::OrderManager;
use std::sync::Arc;
use tracing::{info, error};
use rust_decimal::Decimal;

/// 执行引擎
pub struct ExecutionEngine {
    order_manager: Arc<OrderManager>,
    config: TradingConfig,
}

impl ExecutionEngine {
    pub fn new(order_manager: Arc<OrderManager>, config: TradingConfig) -> Self {
        Self {
            order_manager,
            config,
        }
    }

    /// 执行订单
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
        let commission = total_value * Decimal::from_f64_retain(self.config.default_commission_rate)
            .unwrap_or(Decimal::ZERO);

        order.filled_quantity = order.quantity;
        order.commission = commission;
        order.status = OrderStatus::Filled;
        order.updated_at = chrono::Utc::now();

        self.order_manager.update_order_status(order.order_id, OrderStatus::Filled).await?;
        
        info!("Order {} filled at price {}", order.order_id, execution_price);
        Ok(())
    }

    /// 实盘执行
    async fn real_execution(&self, _order: Order, _market_data: &MarketData) -> Result<()> {
        // TODO: 实现真实交易所API对接
        // 1. 连接交易所API
        // 2. 提交订单
        // 3. 监控订单状态
        // 4. 处理成交回报
        
        error!("Real trading not implemented yet");
        Err(Error::Internal("Real trading not implemented".to_string()))
    }

    /// 计算执行价格（含滑点）
    fn calculate_execution_price(&self, order: &Order, market_data: &MarketData) -> Result<Decimal> {
        let base_price = order.price.unwrap_or(market_data.close);
        let slippage_factor = Decimal::from_f64_retain(self.config.default_slippage)
            .unwrap_or(Decimal::ZERO);

        let slippage_amount = base_price * slippage_factor;

        let execution_price = match order.side {
            quant_common::types::OrderSide::Buy => base_price + slippage_amount,
            quant_common::types::OrderSide::Sell => base_price - slippage_amount,
        };

        Ok(execution_price)
    }
}
