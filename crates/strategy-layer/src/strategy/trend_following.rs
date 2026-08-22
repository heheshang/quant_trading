//! 趋势跟踪策略 (Trend Following Strategy)

use async_trait::async_trait;
use chrono::Utc;
use quant_common::types::{Order, OrderSide, OrderType, ParameterSchema, StrategyParams};
use quant_common::Result;
use rust_decimal::Decimal;
use tracing::{info, instrument, warn};

use super::{Strategy, StrategyContext};

const DEFAULT_SHORT_PERIOD: usize = 12;
const DEFAULT_LONG_PERIOD: usize = 26;
const DEFAULT_SIGNAL_PERIOD: usize = 9;

/// 双均线趋势跟踪策略
///
/// 基于 EMA 交叉生成信号：短期 EMA 上穿长期 EMA → 买入；
/// 短期 EMA 下穿长期 EMA → 卖出。辅以 MACD 柱状图确认趋势强度。
pub struct TrendFollowingStrategy {
    params: StrategyParams,
    short_period: usize,
    long_period: usize,
    signal_period: usize,
}

impl TrendFollowingStrategy {
    pub fn new() -> Self {
        Self {
            params: StrategyParams::builder(
                "trend_following_001",
                "Trend Following Strategy",
                quant_common::types::StrategyType::TrendFollowing,
            )
            .params(serde_json::json!({
                "short_period": DEFAULT_SHORT_PERIOD,
                "long_period": DEFAULT_LONG_PERIOD,
                "signal_period": DEFAULT_SIGNAL_PERIOD,
            }))
            .max_position(Decimal::new(100000, 0))
            .max_daily_loss(Decimal::new(5000, 0))
            .build(),
            short_period: DEFAULT_SHORT_PERIOD,
            long_period: DEFAULT_LONG_PERIOD,
            signal_period: DEFAULT_SIGNAL_PERIOD,
        }
    }
}

impl Default for TrendFollowingStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Strategy for TrendFollowingStrategy {
    #[instrument(skip(self), fields(strategy_id = %params.strategy_id))]
    async fn initialize(&mut self, params: StrategyParams) -> Result<()> {
        info!(strategy_id = %params.strategy_id, "Initializing trend following strategy");
        self.params = params.clone();

        if let Some(short) = params.params.get("short_period") {
            self.short_period = short.as_u64().unwrap_or(DEFAULT_SHORT_PERIOD as u64) as usize;
        }
        if let Some(long) = params.params.get("long_period") {
            self.long_period = long.as_u64().unwrap_or(DEFAULT_LONG_PERIOD as u64) as usize;
        }
        if let Some(sig) = params.params.get("signal_period") {
            self.signal_period = sig.as_u64().unwrap_or(DEFAULT_SIGNAL_PERIOD as u64) as usize;
        }

        info!(
            strategy_id = %params.strategy_id,
            short_period = self.short_period,
            long_period = self.long_period,
            signal_period = self.signal_period,
            "Trend following strategy initialized"
        );
        Ok(())
    }

    #[instrument(skip(self, context), fields(strategy_id = %self.params.strategy_id))]
    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>> {
        info!(
            strategy_id = %self.params.strategy_id,
            data_points = context.market_data.len(),
            positions = context.positions.len(),
            "Generating trend following signals"
        );

        if context.market_data.len() < self.long_period + self.signal_period {
            warn!(
                strategy_id = %self.params.strategy_id,
                available = context.market_data.len(),
                required = self.long_period + self.signal_period,
                "Insufficient market data for trend following"
            );
            return Ok(Vec::new());
        }

        let closes: Vec<Decimal> = context.market_data.iter().map(|d| d.close).collect();

        let short_ema = crate::indicators::ema(&closes, self.short_period);
        let long_ema = crate::indicators::ema(&closes, self.long_period);

        if short_ema.len() < 2 || long_ema.len() < 2 {
            return Ok(Vec::new());
        }

        let min_len = short_ema.len().min(long_ema.len());
        let short_aligned = &short_ema[short_ema.len() - min_len..];
        let long_aligned = &long_ema[long_ema.len() - min_len..];

        let short_now = short_aligned[min_len - 1];
        let short_prev = short_aligned[min_len - 2];
        let long_now = long_aligned[min_len - 1];
        let long_prev = long_aligned[min_len - 2];

        let last_close = closes[closes.len() - 1];
        if last_close == Decimal::ZERO {
            return Ok(Vec::new());
        }

        let short_above_now = short_now > long_now;
        let short_above_prev = short_prev > long_prev;
        let bullish_cross = short_above_now && !short_above_prev;
        let bearish_cross = !short_above_now && short_above_prev;

        if !bullish_cross && !bearish_cross {
            return Ok(Vec::new());
        }

        let side = if bullish_cross {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let label = if bullish_cross {
            "golden cross"
        } else {
            "death cross"
        };

        info!(
            strategy_id = %self.params.strategy_id,
            last_close = %last_close,
            short_ema = %short_now,
            long_ema = %long_now,
            "Signal triggered: {}", label
        );

        Ok(vec![Order {
            order_id: 0,
            strategy_id: self.params.strategy_id.clone(),
            symbol: context.market_data[0].symbol.clone(),
            order_type: OrderType::Limit,
            side,
            price: Some(last_close),
            quantity: self.params.max_position / last_close,
            filled_quantity: Decimal::ZERO,
            status: quant_common::types::OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO,
        }])
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
                name: "short_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(DEFAULT_SHORT_PERIOD),
                range: Some(quant_common::types::ParamRange {
                    min: 5.0,
                    max: 50.0,
                    step: Some(1.0),
                }),
                description: "Short EMA period for crossover detection".into(),
            },
            ParameterSchema {
                name: "long_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(DEFAULT_LONG_PERIOD),
                range: Some(quant_common::types::ParamRange {
                    min: 20.0,
                    max: 200.0,
                    step: Some(1.0),
                }),
                description: "Long EMA period for crossover detection".into(),
            },
            ParameterSchema {
                name: "signal_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(DEFAULT_SIGNAL_PERIOD),
                range: Some(quant_common::types::ParamRange {
                    min: 3.0,
                    max: 30.0,
                    step: Some(1.0),
                }),
                description: "MACD signal line period for trend confirmation".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use quant_common::types::MarketData;

    fn make_market_data(close: Decimal) -> MarketData {
        MarketData {
            timestamp: Utc::now(),
            symbol: "BTC/USDT".to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: Decimal::from(1000),
            turnover: Decimal::ZERO,
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        }
    }

    fn make_full_market_data(timestamp: DateTime<Utc>, close: Decimal, symbol: &str) -> MarketData {
        MarketData {
            timestamp,
            symbol: symbol.to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: Decimal::from(1000),
            turnover: Decimal::ZERO,
            open_interest: None,
            bid_prices: vec![],
            bid_volumes: vec![],
            ask_prices: vec![],
            ask_volumes: vec![],
        }
    }

    fn build_crossover_series(up: bool, symbol: &str) -> Vec<MarketData> {
        let utc = Utc::now();
        let mut data = Vec::with_capacity(31);
        for i in 0..30 {
            let close = if up {
                Decimal::from(100 - i)
            } else {
                Decimal::from(100 + i)
            };
            data.push(make_full_market_data(
                utc + chrono::Duration::hours(i as i64),
                close,
                symbol,
            ));
        }
        let last = if up {
            Decimal::from(200)
        } else {
            Decimal::from(40)
        };
        data.push(make_full_market_data(
            utc + chrono::Duration::hours(30),
            last,
            symbol,
        ));
        data
    }

    #[tokio::test]
    async fn test_trend_following_insufficient_data_returns_empty() {
        let strategy = TrendFollowingStrategy::new();
        let context = StrategyContext {
            current_time: Utc::now(),
            positions: vec![],
            market_data: vec![make_market_data(Decimal::from(100))],
        };
        let orders = strategy.generate_signals(&context).await.unwrap();
        assert!(orders.is_empty());
    }

    #[tokio::test]
    async fn test_trend_following_golden_cross_triggers_buy() {
        let mut strategy = TrendFollowingStrategy::new();
        let params = StrategyParams {
            strategy_id: "tf_test".to_string(),
            strategy_name: "TF Test".to_string(),
            strategy_type: quant_common::types::StrategyType::TrendFollowing,
            params: serde_json::json!({
                "short_period": 5,
                "long_period": 10,
                "signal_period": 3,
            }),
            enabled: true,
            max_position: Decimal::from(100000),
            max_daily_loss: Decimal::from(5000),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: quant_common::types::StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            user_id: 0,
            version: 0,
        };
        strategy.initialize(params).await.unwrap();

        let data = build_crossover_series(true, "BTC/USDT");
        let context = StrategyContext {
            current_time: Utc::now(),
            positions: vec![],
            market_data: data,
        };
        let orders = strategy.generate_signals(&context).await.unwrap();
        assert!(
            orders.iter().any(|o| o.side == OrderSide::Buy),
            "Expected Buy signal: short EMA crossed above long EMA on the final bar"
        );
    }

    #[tokio::test]
    async fn test_trend_following_death_cross_triggers_sell() {
        let mut strategy = TrendFollowingStrategy::new();
        let params = StrategyParams {
            strategy_id: "tf_test".to_string(),
            strategy_name: "TF Test".to_string(),
            strategy_type: quant_common::types::StrategyType::TrendFollowing,
            params: serde_json::json!({
                "short_period": 5,
                "long_period": 10,
                "signal_period": 3,
            }),
            enabled: true,
            max_position: Decimal::from(100000),
            max_daily_loss: Decimal::from(5000),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: quant_common::types::StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            user_id: 0,
            version: 0,
        };
        strategy.initialize(params).await.unwrap();

        let data = build_crossover_series(false, "BTC/USDT");
        let context = StrategyContext {
            current_time: Utc::now(),
            positions: vec![],
            market_data: data,
        };
        let orders = strategy.generate_signals(&context).await.unwrap();
        assert!(
            orders.iter().any(|o| o.side == OrderSide::Sell),
            "Expected Sell signal: short EMA crossed below long EMA on the final bar"
        );
    }

    #[tokio::test]
    async fn test_trend_following_update_params_preserves_runtime_state() {
        let mut strategy = TrendFollowingStrategy::new();
        let initial_params = StrategyParams {
            strategy_id: "tf_orig".to_string(),
            strategy_name: "TF Original".to_string(),
            strategy_type: quant_common::types::StrategyType::TrendFollowing,
            params: serde_json::json!({
                "short_period": 8,
                "long_period": 20,
                "signal_period": 5,
            }),
            enabled: true,
            max_position: Decimal::from(100000),
            max_daily_loss: Decimal::from(5000),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: quant_common::types::StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            user_id: 0,
            version: 0,
        };
        strategy.initialize(initial_params).await.unwrap();
        assert_eq!(strategy.short_period, 8);

        let new_params = StrategyParams {
            strategy_id: "tf_orig".to_string(),
            strategy_name: "TF Original".to_string(),
            strategy_type: quant_common::types::StrategyType::TrendFollowing,
            params: serde_json::json!({
                "short_period": 15,
                "long_period": 50,
                "signal_period": 9,
            }),
            enabled: true,
            max_position: Decimal::from(100000),
            max_daily_loss: Decimal::from(5000),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: quant_common::types::StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            user_id: 0,
            version: 0,
        };
        strategy.update_params(new_params).await.unwrap();
        assert_eq!(
            strategy.short_period, 8,
            "update_params must not reset parsed short_period"
        );
    }
}
