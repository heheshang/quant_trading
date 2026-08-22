//! 订单执行策略（GoF Strategy 模式）。
//!
//! 将 `ExecutionEngine` 中按 paper/real 硬编码分叉的订单执行逻辑，拆分为可插拔的
//! [`ExecutionStrategy`] trait 及两个实现：
//! - [`PaperExecutionStrategy`]：模拟盘撮合（延迟 + 滑点 + 佣金）
//! - [`OkxExecutionStrategy`]：OKX 实盘（下单 + 成交明细回填 + 保守降级）
//!
//! 新增执行模式只需新建策略实现即可，无需改动引擎。

use crate::okx_executor::OkxExecutor;
use crate::order_manager::OrderManager;
use chrono::Utc;
use quant_common::config::TradingConfig;
use quant_common::types::{MarketData, Order, OrderSide, OrderStatus};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{error, info, instrument};

use super::ExecutionResult;

/// 订单执行策略接口。
///
/// 实现应负责完成一笔订单的执行（模拟撮合 / 实盘委托）并返回标准化的
/// [`ExecutionResult`]。`Order` 与 `MarketData` 由引擎传入。
#[async_trait::async_trait]
pub trait ExecutionStrategy: Send + Sync {
    /// 执行一笔订单，返回执行结果。
    async fn execute(&self, order: Order, market_data: &MarketData) -> Result<ExecutionResult>;
}

/// 模拟盘执行策略。
///
/// 按 `simulation_delay_ms` 延迟模拟撮合，以 `default_slippage` 计算滑点成交价，
/// 按 `default_commission_rate` 估算佣金，并将订单状态更新为 `Filled`。
pub struct PaperExecutionStrategy {
    order_manager: Arc<OrderManager>,
    config: TradingConfig,
}

impl PaperExecutionStrategy {
    pub fn new(order_manager: Arc<OrderManager>, config: TradingConfig) -> Self {
        Self {
            order_manager,
            config,
        }
    }

    /// 计算执行价格（含滑点）。
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
            OrderSide::Buy => base_price + slippage_amount,
            OrderSide::Sell => base_price - slippage_amount,
        };

        Ok(execution_price)
    }
}

#[async_trait::async_trait]
impl ExecutionStrategy for PaperExecutionStrategy {
    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side, strategy = "paper"))]
    async fn execute(&self, mut order: Order, market_data: &MarketData) -> Result<ExecutionResult> {
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
}

/// OKX 实盘执行策略。
///
/// 通过 [`OkxExecutor`] 向 OKX 下单，随后查询真实成交明细回填成交价/成交量/状态/手续费；
/// 查询失败时保守回退到订单参数与全额成交假定。
pub struct OkxExecutionStrategy {
    order_manager: Arc<OrderManager>,
    config: TradingConfig,
    okx_executor: Option<Arc<OkxExecutor>>,
}

impl OkxExecutionStrategy {
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
}

#[async_trait::async_trait]
impl ExecutionStrategy for OkxExecutionStrategy {
    #[instrument(skip(self, _market_data), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side, strategy = "okx"))]
    async fn execute(&self, order: Order, _market_data: &MarketData) -> Result<ExecutionResult> {
        match &self.okx_executor {
            Some(executor) => {
                let okx_order_id = executor.execute_order(&order).await.map_err(|e| {
                    error!(order_id = %order.order_id, error = %e, "OKX real execution failed");
                    Error::Internal(format!("OKX execution failed: {}", e))
                })?;

                info!(
                    order_id = %order.order_id,
                    okx_order_id = %okx_order_id,
                    "Order placed on OKX"
                );

                // 拉取真实成交明细（成交价/成交量/状态）；查询失败时回退到订单参数
                let detail = executor
                    .get_order_details(&order.symbol, &okx_order_id)
                    .await;

                let (avg_price, filled_quantity, status, okx_fee) = match detail {
                    Ok(d) => {
                        let avg = d
                            .avg_px
                            .parse::<Decimal>()
                            .unwrap_or_else(|_| order.price.unwrap_or(Decimal::ZERO));
                        let filled = d.acc_fill_sz.parse::<Decimal>().unwrap_or(order.quantity);
                        let status =
                            OkxExecutor::map_okx_state(&d.state).unwrap_or(OrderStatus::Filled);
                        let fee = d.fee.parse::<Decimal>().ok();
                        (avg, filled, status, fee)
                    }
                    Err(_) => {
                        // 查询失败：保守回退（假定全额成交、以订单价成交）
                        (
                            order.price.unwrap_or(Decimal::ZERO),
                            order.quantity,
                            OrderStatus::Filled,
                            None,
                        )
                    }
                };

                // 手续费：优先用 OKX 真实 fee；未捕获时按配置费率估算
                let commission = okx_fee.unwrap_or_else(|| {
                    avg_price
                        * filled_quantity
                        * Decimal::from_f64_retain(self.config.default_commission_rate)
                            .unwrap_or(Decimal::ZERO)
                });

                self.order_manager
                    .update_order_status(order.order_id, status.clone())
                    .await?;

                Ok(ExecutionResult {
                    order_id: order.order_id,
                    strategy_id: order.strategy_id,
                    symbol: order.symbol,
                    filled_quantity,
                    avg_price,
                    commission,
                    status,
                    executed_at: Utc::now(),
                })
            }
            None => {
                error!("No OKX executor configured for real execution");
                Err(Error::Internal("No OKX executor configured".to_string()))
            }
        }
    }
}
