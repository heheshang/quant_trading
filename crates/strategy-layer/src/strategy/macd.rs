//! MACD 交叉策略 (MACD Crossover)
//!
//! 趋势/动量策略：MACD 线由上穿信号线（柱状图由负转正）时买入，
//! 下穿（柱状图由正转负）时卖出。

use async_trait::async_trait;
use chrono::Utc;
use quant_common::types::{Order, OrderSide, OrderType, ParameterSchema, StrategyParams, StrategyType};
use quant_common::Result;
use rust_decimal::Decimal;
use tracing::{info, instrument, warn};

use super::{Strategy, StrategyContext};

const DEFAULT_FAST: usize = 12;
const DEFAULT_SLOW: usize = 26;
const DEFAULT_SIGNAL: usize = 9;

/// MACD 交叉策略
pub struct MacdStrategy {
    params: StrategyParams,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
}

impl MacdStrategy {
    #[instrument]
    pub fn new() -> Self {
        Self {
            params: StrategyParams::builder(
                "macd-default",
                "MACD 交叉策略",
                StrategyType::Macd,
            )
            .params(serde_json::json!({
                "fast_period": DEFAULT_FAST,
                "slow_period": DEFAULT_SLOW,
                "signal_period": DEFAULT_SIGNAL,
            }))
            .max_position(Decimal::from(100000))
            .max_daily_loss(Decimal::from(5000))
            .build(),
            fast_period: DEFAULT_FAST,
            slow_period: DEFAULT_SLOW,
            signal_period: DEFAULT_SIGNAL,
        }
    }
}

#[async_trait]
impl Strategy for MacdStrategy {
    #[instrument(skip(self), fields(strategy_id = %params.strategy_id))]
    async fn initialize(&mut self, params: StrategyParams) -> Result<()> {
        self.params = params;
        if let Some(v) = self.params.params.get("fast_period") {
            self.fast_period = v.as_u64().unwrap_or(DEFAULT_FAST as u64) as usize;
        }
        if let Some(v) = self.params.params.get("slow_period") {
            self.slow_period = v.as_u64().unwrap_or(DEFAULT_SLOW as u64) as usize;
        }
        if let Some(v) = self.params.params.get("signal_period") {
            self.signal_period = v.as_u64().unwrap_or(DEFAULT_SIGNAL as u64) as usize;
        }
        info!(strategy_id = %self.params.strategy_id, "Strategy initialized");
        Ok(())
    }

    #[instrument(skip(self, context), fields(strategy_id = %self.params.strategy_id))]
    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>> {
        info!(
            strategy_id = %self.params.strategy_id,
            data_points = context.market_data.len(),
            "Generating signals"
        );

        let closes: Vec<Decimal> = context.market_data.iter().map(|d| d.close).collect();
        if closes.len() < self.slow_period + self.signal_period {
            warn!(
                strategy_id = %self.params.strategy_id,
                available = closes.len(),
                required = self.slow_period + self.signal_period,
                "Insufficient market data for MACD"
            );
            return Ok(Vec::new());
        }

        let ema_fast = crate::indicators::ema(&closes, self.fast_period);
        let ema_slow = crate::indicators::ema(&closes, self.slow_period);
        if ema_fast.is_empty() || ema_slow.is_empty() {
            return Ok(Vec::new());
        }

        // Align the two EMA series to the same close index (EMA_starts later).
        let offset = self.slow_period - self.fast_period;
        let macd: Vec<Decimal> = (0..ema_slow.len())
            .map(|i| ema_fast[i + offset] - ema_slow[i])
            .collect();
        let signal = crate::indicators::ema(&macd, self.signal_period);
        if signal.len() < 2 {
            return Ok(Vec::new());
        }

        let hist_last = macd[macd.len() - 1] - signal[signal.len() - 1];
        let hist_prev = macd[macd.len() - 2] - signal[signal.len() - 2];
        let last_close = closes[closes.len() - 1];
        if last_close <= Decimal::ZERO {
            return Ok(Vec::new());
        }

        let mut orders = Vec::new();
        // Buy when MACD histogram crosses above zero (bullish momentum).
        if hist_prev <= Decimal::ZERO && hist_last > Decimal::ZERO {
            info!(strategy_id = %self.params.strategy_id, %last_close, "MACD buy signal");
            orders.push(Order { order_id: 0,
            strategy_id: self.params.strategy_id.clone(),
            symbol: context.market_data[0].symbol.clone(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(last_close),
            quantity: self.params.max_position / last_close,
            filled_quantity: Decimal::ZERO,
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO, exchange: "paper".to_string(), });
        } else if hist_prev >= Decimal::ZERO && hist_last < Decimal::ZERO {
            info!(strategy_id = %self.params.strategy_id, %last_close, "MACD sell signal");
            orders.push(Order { order_id: 0,
            strategy_id: self.params.strategy_id.clone(),
            symbol: context.market_data[0].symbol.clone(),
            order_type: OrderType::Limit,
            side: OrderSide::Sell,
            price: Some(last_close),
            quantity: self.params.max_position / last_close,
            filled_quantity: Decimal::ZERO,
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO, exchange: "paper".to_string(), });
        }

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
                name: "fast_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(12),
                range: Some(quant_common::types::ParamRange {
                    min: 2.0,
                    max: 50.0,
                    step: Some(1.0),
                }),
                description: "Fast EMA period".into(),
            },
            ParameterSchema {
                name: "slow_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(26),
                range: Some(quant_common::types::ParamRange {
                    min: 5.0,
                    max: 100.0,
                    step: Some(1.0),
                }),
                description: "Slow EMA period".into(),
            },
            ParameterSchema {
                name: "signal_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(9),
                range: Some(quant_common::types::ParamRange {
                    min: 2.0,
                    max: 50.0,
                    step: Some(1.0),
                }),
                description: "Signal EMA period".into(),
            },
        ]
    }
}

impl Default for MacdStrategy {
    fn default() -> Self {
        Self::new()
    }
}
