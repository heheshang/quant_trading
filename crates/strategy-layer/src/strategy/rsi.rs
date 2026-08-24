//! RSI 反转策略 (RSI Reversal)
//!
//! 均值回归策略：RSI 低于超卖阈值时买入，高于超买阈值时卖出。

use async_trait::async_trait;
use chrono::Utc;
use quant_common::types::{Order, OrderSide, OrderType, ParameterSchema, StrategyParams, StrategyType};
use quant_common::Result;
use rust_decimal::Decimal;
use tracing::{info, instrument, warn};

use super::{Strategy, StrategyContext};

const DEFAULT_PERIOD: u64 = 14;
const DEFAULT_OVERSOLD: u64 = 30;
const DEFAULT_OVERBOUGHT: u64 = 70;

/// RSI 反转策略
pub struct RsiStrategy {
    params: StrategyParams,
    period: usize,
    oversold: Decimal,
    overbought: Decimal,
}

impl RsiStrategy {
    #[instrument]
    pub fn new() -> Self {
        Self {
            params: StrategyParams::builder(
                "rsi-default",
                "RSI 反转策略",
                StrategyType::Rsi,
            )
            .params(serde_json::json!({
                "period": DEFAULT_PERIOD,
                "oversold": DEFAULT_OVERSOLD,
                "overbought": DEFAULT_OVERBOUGHT,
            }))
            .max_position(Decimal::from(100000))
            .max_daily_loss(Decimal::from(5000))
            .build(),
            period: DEFAULT_PERIOD as usize,
            oversold: Decimal::from(DEFAULT_OVERSOLD),
            overbought: Decimal::from(DEFAULT_OVERBOUGHT),
        }
    }
}

#[async_trait]
impl Strategy for RsiStrategy {
    #[instrument(skip(self), fields(strategy_id = %params.strategy_id))]
    async fn initialize(&mut self, params: StrategyParams) -> Result<()> {
        self.params = params;
        if let Some(v) = self.params.params.get("period") {
            self.period = v.as_u64().unwrap_or(DEFAULT_PERIOD) as usize;
        }
        if let Some(v) = self.params.params.get("oversold") {
            self.oversold = Decimal::from(v.as_u64().unwrap_or(DEFAULT_OVERSOLD));
        }
        if let Some(v) = self.params.params.get("overbought") {
            self.overbought = Decimal::from(v.as_u64().unwrap_or(DEFAULT_OVERBOUGHT));
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
        if closes.len() < self.period + 1 {
            warn!(
                strategy_id = %self.params.strategy_id,
                available = closes.len(),
                required = self.period + 1,
                "Insufficient market data for RSI"
            );
            return Ok(Vec::new());
        }

        let rsi_values = crate::indicators::rsi(&closes, self.period);
        let last_rsi = match rsi_values.last() {
            Some(v) => *v,
            None => return Ok(Vec::new()),
        };
        let last_close = closes[closes.len() - 1];
        if last_close <= Decimal::ZERO {
            return Ok(Vec::new());
        }

        let mut orders = Vec::new();
        // Buy when RSI is oversold.
        if last_rsi < self.oversold {
            info!(strategy_id = %self.params.strategy_id, %last_rsi, "RSI buy signal (oversold)");
            orders.push(Order {
                order_id: 0,
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
                slippage: Decimal::ZERO,
            });
        } else if last_rsi > self.overbought {
            info!(strategy_id = %self.params.strategy_id, %last_rsi, "RSI sell signal (overbought)");
            orders.push(Order {
                order_id: 0,
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
                slippage: Decimal::ZERO,
            });
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
                name: "period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(14),
                range: Some(quant_common::types::ParamRange {
                    min: 2.0,
                    max: 100.0,
                    step: Some(1.0),
                }),
                description: "RSI lookback period".into(),
            },
            ParameterSchema {
                name: "oversold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(30),
                range: Some(quant_common::types::ParamRange {
                    min: 10.0,
                    max: 45.0,
                    step: Some(1.0),
                }),
                description: "Oversold threshold (buy below)".into(),
            },
            ParameterSchema {
                name: "overbought".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(70),
                range: Some(quant_common::types::ParamRange {
                    min: 55.0,
                    max: 90.0,
                    step: Some(1.0),
                }),
                description: "Overbought threshold (sell above)".into(),
            },
        ]
    }
}

impl Default for RsiStrategy {
    fn default() -> Self {
        Self::new()
    }
}
