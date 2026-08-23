use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use quant_common::types::{Account, MarketData, Position};
use risk_engine::PreTradeRiskChecker;
use rust_decimal::Decimal;
use strategy_engine::pipeline::{OrderExecStep, RiskCheckStep};
use strategy_engine::signals::Signal;
use strategy_engine::traits::{OrderExecError, OrderExecutor, RiskCheckError, RiskChecker};
use trading_engine::ExecutionEngine;

use crate::order_processor::OrderProcessor;

/// Wraps [`PreTradeRiskChecker`] to implement the strategy-layer [`RiskChecker`] trait.
struct RiskLayerChecker {
    inner: Arc<PreTradeRiskChecker>,
}

impl RiskLayerChecker {
    fn new(inner: Arc<PreTradeRiskChecker>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl RiskChecker for RiskLayerChecker {
    async fn check(&self, signal: &Signal) -> Result<(), RiskCheckError> {
        let order = signal.to_order(&signal.strategy_id).ok_or_else(|| {
            RiskCheckError::Internal("Cannot convert signal to order".to_string())
        })?;

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

        self.inner
            .check_order(&order, &account, &positions)
            .map_err(|e| RiskCheckError::Rejected(e.to_string()))
    }
}

/// Wraps [`ExecutionEngine`] to implement the strategy-layer [`OrderExecutor`] trait.
struct TradingLayerExecutor {
    inner: Arc<ExecutionEngine>,
}

impl TradingLayerExecutor {
    fn new(inner: Arc<ExecutionEngine>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl OrderExecutor for TradingLayerExecutor {
    async fn execute(&self, signal: &Signal) -> Result<String, OrderExecError> {
        let order = signal.to_order(&signal.strategy_id).ok_or_else(|| {
            OrderExecError::Internal("Cannot convert signal to order".to_string())
        })?;

        let market_data = MarketData {
            symbol: order.symbol.clone(),
            timestamp: Utc::now(),
            open: order.price.unwrap_or(Decimal::ZERO),
            high: order.price.unwrap_or(Decimal::ZERO),
            low: order.price.unwrap_or(Decimal::ZERO),
            close: order.price.unwrap_or(Decimal::ZERO),
            volume: Decimal::ZERO,
            turnover: Decimal::ZERO,
            open_interest: None,
            bid_prices: Vec::new(),
            bid_volumes: Vec::new(),
            ask_prices: Vec::new(),
            ask_volumes: Vec::new(),
        };

        self.inner
            .execute_order(order, &market_data)
            .await
            .map(|result| result.order_id.to_string())
            .map_err(|e| OrderExecError::Internal(e.to_string()))
    }
}

/// Create a [`RiskCheckStep`] backed by a real [`PreTradeRiskChecker`].
pub fn make_risk_check_step(checker: Arc<PreTradeRiskChecker>) -> RiskCheckStep {
    RiskCheckStep::new(Box::new(RiskLayerChecker::new(checker)))
}

/// Create an [`OrderExecStep`] backed by a real [`ExecutionEngine`].
pub fn make_order_exec_step(engine: Arc<ExecutionEngine>) -> OrderExecStep {
    OrderExecStep::new(Box::new(TradingLayerExecutor::new(engine)))
}

/// Wraps [`OrderProcessor`] to implement the strategy-layer [`OrderExecutor`] trait.
///
/// Routes each pipeline order through the full [`OrderProcessor::place_order`]
/// use-case (market data resolve → pre-trade risk → in-memory submission →
/// persistence → event descriptor → async execution), so strategy-generated
/// orders are placed exactly like manually-submitted ones. When pre-trade risk
/// is enabled, `place_order` enforces it with live account data (fail-closed).
struct OrderProcessorExecutor {
    inner: Arc<OrderProcessor>,
}

impl OrderProcessorExecutor {
    fn new(inner: Arc<OrderProcessor>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl OrderExecutor for OrderProcessorExecutor {
    async fn execute(&self, signal: &Signal) -> Result<String, OrderExecError> {
        let order = signal.to_order(&signal.strategy_id).ok_or_else(|| {
            OrderExecError::Internal("Cannot convert signal to order".to_string())
        })?;

        let placement = self
            .inner
            .place_order(order)
            .await
            .map_err(|e| OrderExecError::Internal(e.to_string()))?;

        Ok(placement.order_id.to_string())
    }
}

/// Create an [`OrderExecStep`] backed by a real [`OrderProcessor`].
///
/// Use this instead of [`make_order_exec_step`] when the pipeline should route
/// strategy orders through the full single-placement use-case (risk + paper /
/// real execution + persistence) rather than a bare [`ExecutionEngine`].
pub fn make_order_processor_exec_step(processor: Arc<OrderProcessor>) -> OrderExecStep {
    OrderExecStep::new(Box::new(OrderProcessorExecutor::new(processor)))
}
