//! 均值回归策略 (Mean Reversion Strategy)

use async_trait::async_trait;
use chrono::Utc;
use quant_common::types::{Order, OrderSide, OrderType, ParameterSchema, StrategyParams};
use quant_common::Result;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use tracing::{info, instrument, warn};

use super::{Strategy, StrategyContext};

const DEFAULT_ENTRY_THRESHOLD: f64 = 2.0;
const RSI_OVERSOLD: u32 = 30;
const RSI_OVERBOUGHT: u32 = 70;

/// 均值回归策略
pub struct MeanReversionStrategy {
    params: StrategyParams,
    lookback_period: usize,
    entry_threshold: f64,
    exit_threshold: f64,
}

impl MeanReversionStrategy {
    #[instrument]
    pub fn new() -> Self {
        Self {
            params: StrategyParams::builder(
                "mean_reversion_001",
                "Mean Reversion Strategy",
                quant_common::types::StrategyType::MeanReversion,
            )
            .params(serde_json::json!({
                "lookback_period": 20,
                "entry_threshold": 2.0,
                "exit_threshold": 0.5,
            }))
            .max_position(Decimal::new(100000, 0))
            .max_daily_loss(Decimal::new(5000, 0))
            .build(),
            lookback_period: 20,
            entry_threshold: DEFAULT_ENTRY_THRESHOLD,
            exit_threshold: 0.5,
        }
    }
}

#[async_trait]
impl Strategy for MeanReversionStrategy {
    #[instrument(skip(self), fields(strategy_id = %params.strategy_id))]
    async fn initialize(&mut self, params: StrategyParams) -> Result<()> {
        info!(strategy_id = %params.strategy_id, "Initializing strategy");
        self.params = params.clone();

        if let Some(lookback) = params.params.get("lookback_period") {
            self.lookback_period = lookback.as_u64().unwrap_or(20) as usize;
        }
        if let Some(entry) = params.params.get("entry_threshold") {
            self.entry_threshold = entry.as_f64().unwrap_or(DEFAULT_ENTRY_THRESHOLD);
        }
        if let Some(exit) = params.params.get("exit_threshold") {
            self.exit_threshold = exit.as_f64().unwrap_or(0.5);
        }

        info!(strategy_id = %params.strategy_id, "Strategy initialized");
        Ok(())
    }

    #[instrument(skip(self, context), fields(strategy_id = %self.params.strategy_id))]
    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>> {
        info!(
            strategy_id = %self.params.strategy_id,
            data_points = context.market_data.len(),
            positions = context.positions.len(),
            "Generating signals"
        );

        if context.market_data.len() < self.lookback_period {
            warn!(
                strategy_id = %self.params.strategy_id,
                available = context.market_data.len(),
                required = self.lookback_period,
                "Insufficient market data for signal generation"
            );
            return Ok(Vec::new());
        }

        let closes: Vec<Decimal> = context.market_data.iter().map(|d| d.close).collect();

        // Compute SMA
        let sma_values = crate::indicators::sma(&closes, self.lookback_period);
        if sma_values.is_empty() {
            return Ok(Vec::new());
        }
        let last_sma = sma_values[sma_values.len() - 1];

        // Compute RSI (period 14, standard)
        let rsi_values = crate::indicators::rsi(&closes, 14);
        if rsi_values.is_empty() {
            return Ok(Vec::new());
        }
        let last_rsi = rsi_values[rsi_values.len() - 1];

        // Compute standard deviation over the last lookback_period window
        let window_start = closes.len() - self.lookback_period;
        let window = &closes[window_start..];
        let variance: Decimal = window
            .iter()
            .map(|&x| (x - last_sma) * (x - last_sma))
            .sum::<Decimal>()
            / Decimal::from(self.lookback_period);
        let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);

        if std_dev == Decimal::ZERO {
            info!(strategy_id = %self.params.strategy_id, "Std dev is zero, no signals generated");
            return Ok(Vec::new());
        }

        let last_close = closes[closes.len() - 1];
        let entry_threshold_dec = Decimal::from_f64(self.entry_threshold)
            .unwrap_or(Decimal::from_f64(DEFAULT_ENTRY_THRESHOLD).unwrap());

        let mut orders = Vec::new();

        // Buy signal: price below SMA by > entry_threshold stddev AND RSI oversold
        let symbol = context.market_data[0].symbol.clone();
        if (last_sma - last_close) / std_dev > entry_threshold_dec
            && last_rsi < Decimal::from(RSI_OVERSOLD)
        {
            info!(
                strategy_id = %self.params.strategy_id,
                last_close = %last_close,
                last_sma = %last_sma,
                last_rsi = %last_rsi,
                "Buy signal triggered: mean reversion entry"
            );
            let quantity = super::net_quantity(context, &symbol, OrderSide::Buy, self.params.max_position, last_close);
            if quantity > Decimal::ZERO {
                orders.push(Order { order_id: 0,
                    strategy_id: self.params.strategy_id.clone(),
                    symbol: symbol.clone(),
                    order_type: OrderType::Limit,
                    side: OrderSide::Buy,
                    price: Some(last_close),
                    quantity,
                    filled_quantity: Decimal::ZERO,
                    status: quant_common::types::OrderStatus::Pending,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    commission: Decimal::ZERO,
                    slippage: Decimal::ZERO, exchange: "paper".to_string() });
            }
        }

        // Sell signal: price above SMA by > entry_threshold stddev AND RSI overbought
        if (last_close - last_sma) / std_dev > entry_threshold_dec
            && last_rsi > Decimal::from(RSI_OVERBOUGHT)
        {
            info!(
                strategy_id = %self.params.strategy_id,
                last_close = %last_close,
                last_sma = %last_sma,
                last_rsi = %last_rsi,
                "Sell signal triggered: mean reversion entry"
            );
            let quantity = super::net_quantity(context, &symbol, OrderSide::Sell, self.params.max_position, last_close);
            if quantity > Decimal::ZERO {
                orders.push(Order { order_id: 0,
                    strategy_id: self.params.strategy_id.clone(),
                    symbol: symbol.clone(),
                    order_type: OrderType::Limit,
                    side: OrderSide::Sell,
                    price: Some(last_close),
                    quantity,
                    filled_quantity: Decimal::ZERO,
                    status: quant_common::types::OrderStatus::Pending,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    commission: Decimal::ZERO,
                    slippage: Decimal::ZERO, exchange: "paper".to_string() });
            }
        }

        info!(
            strategy_id = %self.params.strategy_id,
            orders = orders.len(),
            "Signal generation complete"
        );
        Ok(orders)
    }

    fn name(&self) -> &str {
        &self.params.strategy_name
    }

    fn params(&self) -> &StrategyParams {
        &self.params
    }

    fn params_mut(&mut self) -> &mut StrategyParams {
        &mut self.params
    }

    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        vec![
            ParameterSchema {
                name: "lookback_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(20),
                range: Some(quant_common::types::ParamRange {
                    min: 5.0,
                    max: 100.0,
                    step: Some(1.0),
                }),
                description: "Lookback period for mean reversion calculation".into(),
            },
            ParameterSchema {
                name: "entry_threshold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(2.0),
                range: Some(quant_common::types::ParamRange {
                    min: 0.5,
                    max: 5.0,
                    step: Some(0.1),
                }),
                description: "Entry threshold in standard deviations".into(),
            },
            ParameterSchema {
                name: "exit_threshold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(0.5),
                range: Some(quant_common::types::ParamRange {
                    min: 0.1,
                    max: 3.0,
                    step: Some(0.1),
                }),
                description: "Exit threshold in standard deviations".into(),
            },
        ]
    }
}

impl Default for MeanReversionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
