//! 订单执行引擎（GoF Strategy + Observer 模式）。
//!
//! [`ExecutionEngine`] 持有可插拔的 [`ExecutionStrategy`]，`execute_order` 仅委托到策略，
//! 不再包含 paper/real 硬编码分支；执行成功后通过 [`ExecutionCallback`] 通知观察者。

use crate::binance_executor::BinanceExecutor;
use crate::live_exchange::LiveExchange;
use crate::order_manager::OrderManager;
use chrono::{DateTime, Utc};
use quant_common::config::TradingConfig;
use quant_common::types::{MarketData, Order, OrderStatus};
use quant_common::Result;
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument};

mod strategy;

#[cfg(test)]
mod tests;

pub use strategy::{BinanceExecutionStrategy, ExecutionStrategy, PaperExecutionStrategy};

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
    strategy: Arc<dyn ExecutionStrategy>,
    callbacks: Vec<Box<dyn ExecutionCallback>>,
}

enum LiveExecutor {
    Binance(Option<Arc<BinanceExecutor>>),
}

impl ExecutionEngine {
    /// 构建一个以 Binance 为实盘执行器、随配置切换 paper/real 的引擎。
    ///
    /// 纸面路径（`enable_paper_trading=true`）忽略实盘执行器，因而 `BinanceExecutor`
    /// 是一个可替换的抽象而非纸面路径的耦合。
    pub fn new_binance(
        order_manager: Arc<OrderManager>,
        config: TradingConfig,
        binance_executor: Option<Arc<BinanceExecutor>>,
    ) -> Self {
        Self::with_executor(
            order_manager,
            config,
            LiveExecutor::Binance(binance_executor),
        )
    }

    /// 依据配置一次性选择执行策略（paper / binance），运行时不再分支。
    fn with_executor(
        order_manager: Arc<OrderManager>,
        config: TradingConfig,
        live: LiveExecutor,
    ) -> Self {
        let strategy: Arc<dyn ExecutionStrategy> = if config.enable_paper_trading {
            Arc::new(PaperExecutionStrategy::new(order_manager, config))
        } else {
            match live {
                LiveExecutor::Binance(ex) => Arc::new(BinanceExecutionStrategy::new(
                    order_manager,
                    config,
                    ex.map(|e| e as Arc<dyn LiveExchange>),
                )),
            }
        };

        Self {
            strategy,
            callbacks: Vec::new(),
        }
    }

    /// 注册执行回调
    pub fn register_callback(&mut self, callback: Box<dyn ExecutionCallback>) {
        self.callbacks.push(callback);
    }

    /// 执行订单
    #[instrument(skip(self, market_data), fields(order_id = %order.order_id, symbol = %order.symbol, side = ?order.side))]
    pub async fn execute_order(
        &self,
        order: Order,
        market_data: &MarketData,
    ) -> Result<ExecutionResult> {
        info!("Executing order: {:?}", order.order_id);

        let result = self.strategy.execute(order, market_data).await;

        // 仅当执行成功时通知回调
        if let Ok(ref r) = result {
            for cb in &self.callbacks {
                cb.on_order_executed(r).await;
            }
        }

        result
    }

    /// 立即撮合（跳过模拟延迟），供纸面调度器在订单到期时调用。
    pub async fn fill_order(
        &self,
        order: Order,
        market_data: &MarketData,
    ) -> Result<ExecutionResult> {
        self.strategy.fill_now(order, market_data).await
    }
}
