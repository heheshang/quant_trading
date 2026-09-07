//! 订单执行策略（GoF Strategy 模式）。
//!
//! 将 `ExecutionEngine` 中按 paper/real 硬编码分叉的订单执行逻辑，拆分为可插拔的
//! [`ExecutionStrategy`] trait 及两个实现：
//! - [`PaperExecutionStrategy`]：模拟盘撮合（延迟 + 滑点 + 佣金）
//! - [`BinanceExecutionStrategy`]：Binance 实盘（下单 + 成交明细回填 + 保守降级）
//!
//! 新增执行模式只需新建策略实现即可，无需改动引擎。

use crate::live_exchange::LiveExchange;
use crate::order_manager::OrderManager;
use chrono::Utc;
use quant_common::config::TradingConfig;
use quant_common::types::{MarketData, Order, OrderSide, OrderStatus, OrderType};
use quant_common::{Error, Result};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

use super::ExecutionResult;

/// 订单执行策略接口。
///
/// 实现应负责完成一笔订单的执行（模拟撮合 / 实盘委托）并返回标准化的
/// [`ExecutionResult`]。`Order` 与 `MarketData` 由引擎传入。
#[async_trait::async_trait]
pub trait ExecutionStrategy: Send + Sync {
    /// 执行一笔订单，返回执行结果。
    async fn execute(&self, order: Order, market_data: &MarketData) -> Result<ExecutionResult>;

    /// 立即执行并提交内存状态（跳过模拟延迟）。
    async fn fill_now(&self, order: Order, market_data: &MarketData) -> Result<ExecutionResult> {
        self.execute(order, market_data).await
    }

    /// 仅计算纸面成交；实盘策略不支持该入口，不能隐式发出委托。
    async fn calculate_paper_fill(
        &self,
        _order: Order,
        _market_data: &MarketData,
    ) -> Result<ExecutionResult> {
        Err(Error::Trading(
            "Paper fill calculation requires a paper strategy".to_string(),
        ))
    }
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
        if order.quantity <= Decimal::ZERO || market_data.close <= Decimal::ZERO {
            return Err(Error::Validation(
                "Order quantity and market price must be positive".to_string(),
            ));
        }
        if order.price.is_some_and(|price| price <= Decimal::ZERO) {
            return Err(Error::Validation(
                "Order price must be positive".to_string(),
            ));
        }
        let limit = if order.order_type == OrderType::Limit {
            let limit = order
                .price
                .ok_or_else(|| Error::Validation("Limit order requires a price".to_string()))?;
            let touched = match order.side {
                OrderSide::Buy => market_data.close <= limit,
                OrderSide::Sell => market_data.close >= limit,
            };
            if !touched {
                return Err(Error::Trading(
                    "Limit price has not been reached".to_string(),
                ));
            }
            Some(limit)
        } else {
            None
        };
        // 以市价为成交基准（非限价），避免「限价±滑点」永远劣于限价。
        let base_price = market_data.close;
        let slippage_factor =
            Decimal::from_f64_retain(self.config.default_slippage).unwrap_or(Decimal::ZERO);

        let slippage_amount = base_price * slippage_factor;

        let mut execution_price = match order.side {
            OrderSide::Buy => base_price + slippage_amount,
            OrderSide::Sell => base_price - slippage_amount,
        };
        // 限价单：成交价不得劣于限价（买入 ≤ limit，卖出 ≥ limit）。
        if let Some(limit) = limit {
            execution_price = match order.side {
                OrderSide::Buy => execution_price.min(limit),
                OrderSide::Sell => execution_price.max(limit),
            };
        }
        if execution_price <= Decimal::ZERO {
            return Err(Error::Validation(
                "Execution price must be positive".to_string(),
            ));
        }

        Ok(execution_price)
    }
}

#[async_trait::async_trait]
impl ExecutionStrategy for PaperExecutionStrategy {
    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side, strategy = "paper"))]
    async fn execute(&self, order: Order, market_data: &MarketData) -> Result<ExecutionResult> {
        let delay_ms = self.config.simulation_delay_ms;
        if delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
        self.fill_now(order, market_data).await
    }

    /// 立即撮合并提交内存事实；仅 DB 恢复单缺失于内存时允许继续。
    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol, strategy = "paper"))]
    async fn fill_now(&self, order: Order, market_data: &MarketData) -> Result<ExecutionResult> {
        let result = self.calculate_paper_fill(order, market_data).await?;
        match self.order_manager.apply_execution_result(&result).await {
            Ok(()) | Err(Error::NotFound(_)) => {}
            Err(error) => return Err(error),
        }
        monitor_engine::MetricsCollector::inc_orders_filled();
        info!(
            "Order {} filled at price {}",
            result.order_id, result.avg_price
        );
        Ok(result)
    }

    async fn calculate_paper_fill(
        &self,
        order: Order,
        market_data: &MarketData,
    ) -> Result<ExecutionResult> {
        if !order.status.is_active() {
            return Err(Error::Trading(format!(
                "Order {} is terminal",
                order.order_id
            )));
        }
        let execution_price = self.calculate_execution_price(&order, market_data)?;
        let remaining_quantity = order.quantity - order.filled_quantity;
        if order.filled_quantity < Decimal::ZERO || remaining_quantity <= Decimal::ZERO {
            return Err(Error::Validation(
                "Order must have a positive remaining quantity".to_string(),
            ));
        }
        let commission = order.commission
            + execution_price
                * remaining_quantity
                * Decimal::from_f64_retain(self.config.default_commission_rate)
                    .unwrap_or(Decimal::ZERO);
        Ok(ExecutionResult {
            order_id: order.order_id,
            strategy_id: order.strategy_id,
            symbol: order.symbol,
            filled_quantity: order.quantity,
            avg_price: execution_price,
            commission,
            status: OrderStatus::Filled,
            executed_at: Utc::now(),
        })
    }
}

/// 实盘订单执行核心逻辑。
///
/// 下单成功后查询真实成交明细回填成交价/成交量/状态/手续费；查询失败时保留
/// Submitted 和零成交，不重复提交委托。`label` 仅用于日志与错误文案，经 [`LiveExchange`] 抽象
/// 由 Binance 策略消费。
async fn execute_live(
    order: Order,
    order_manager: &Arc<OrderManager>,
    config: &TradingConfig,
    executor: Option<&Arc<dyn LiveExchange>>,
    label: &str,
) -> Result<ExecutionResult> {
    let Some(executor) = executor else {
        error!("No {} executor configured for real execution", label);
        return Err(Error::Internal(format!("No {} executor configured", label)));
    };

    let live_order_id = executor.execute_order(&order).await.map_err(|e| {
        error!(order_id = %order.order_id, error = %e, "{} real execution failed", label);
        Error::Internal(format!("{} execution failed: {}", label, e))
    })?;

    info!(
        order_id = %order.order_id,
        live_order_id = %live_order_id,
        "Order placed on {}",
        label
    );

    // 拉取真实成交明细；查询失败只代表成交事实未知，不代表委托失败。
    let detail = executor
        .get_order_details(&order.symbol, &live_order_id)
        .await;

    let (avg_price, filled_quantity, status, fee) = match detail {
        Ok(d) => (d.avg_price, d.filled_quantity, d.status, d.fee),
        Err(error) => {
            warn!(order_id = %order.order_id, live_order_id = %live_order_id, error = %error,
                "{} accepted order but fill details are unavailable; retaining Submitted", label);
            (Decimal::ZERO, Decimal::ZERO, OrderStatus::Submitted, None)
        }
    };

    // 手续费：优先用交易所真实 fee；未捕获时按配置费率估算
    let commission = fee.unwrap_or_else(|| {
        avg_price
            * filled_quantity
            * Decimal::from_f64_retain(config.default_commission_rate).unwrap_or(Decimal::ZERO)
    });

    let result = ExecutionResult {
        order_id: order.order_id,
        strategy_id: order.strategy_id,
        symbol: order.symbol,
        filled_quantity,
        avg_price,
        commission,
        status,
        executed_at: Utc::now(),
    };
    order_manager.apply_execution_result(&result).await?;
    Ok(result)
}

/// Binance 实盘执行策略。
///
/// 经 [`LiveExchange`] 抽象向 Binance 下单，随后查询真实成交明细回填成交价/成交量/状态/手续费；
/// 查询失败时保留 Submitted 和零成交事实，不重复提交。
pub struct BinanceExecutionStrategy {
    order_manager: Arc<OrderManager>,
    config: TradingConfig,
    live_exchange: Option<Arc<dyn LiveExchange>>,
}

impl BinanceExecutionStrategy {
    pub fn new(
        order_manager: Arc<OrderManager>,
        config: TradingConfig,
        live_exchange: Option<Arc<dyn LiveExchange>>,
    ) -> Self {
        Self {
            order_manager,
            config,
            live_exchange,
        }
    }
}

#[async_trait::async_trait]
impl ExecutionStrategy for BinanceExecutionStrategy {
    #[instrument(skip(self, _market_data), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side, strategy = "binance"))]
    async fn execute(&self, order: Order, _market_data: &MarketData) -> Result<ExecutionResult> {
        execute_live(
            order,
            &self.order_manager,
            &self.config,
            self.live_exchange.as_ref(),
            "Binance",
        )
        .await
    }
}
