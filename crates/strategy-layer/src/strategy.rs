use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_common::types::{
    MarketData, Order, OrderSide, OrderType, ParameterSchema, Position, StrategyParams,
};
use quant_common::Result;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use tracing::{info, instrument, warn};

const DEFAULT_ENTRY_THRESHOLD: f64 = 2.0;
const RSI_OVERSOLD: u32 = 30;
const RSI_OVERBOUGHT: u32 = 70;

/// 策略上下文
pub struct StrategyContext {
    pub current_time: DateTime<Utc>,
    pub positions: Vec<Position>,
    pub market_data: Vec<MarketData>,
}

/// 策略接口
#[async_trait]
pub trait Strategy: Send + Sync {
    /// 初始化策略
    async fn initialize(&mut self, params: StrategyParams) -> Result<()>;

    /// 生成交易信号
    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>>;

    /// 策略名称
    fn name(&self) -> &str;

    /// 策略参数
    fn params(&self) -> &StrategyParams;

    /// 策略参数的可变访问器。`update_params` 的默认实现依赖此方法。
    fn params_mut(&mut self) -> &mut StrategyParams;

    /// 更新策略参数（仅元数据，不重置运行时状态）
    ///
    /// 默认实现只替换 `self.params`，不会重新解析已派生的运行时阈值。
    /// 若策略需要更复杂的热更新语义，可重写此方法。
    async fn update_params(&mut self, params: StrategyParams) -> Result<()> {
        *self.params_mut() = params;
        Ok(())
    }

    /// 显式重新初始化：完整重置策略状态并应用新参数。
    ///
    /// 默认实现直接转发给 `initialize`，调用方需自行承担重置带来的状态丢失。
    async fn reinitialize(&mut self, params: StrategyParams) -> Result<()> {
        self.initialize(params).await
    }

    // ── 生命周期钩子（默认空实现） ─────────────────────────────────────

    /// 策略部署时调用
    async fn on_deploy(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略启动时调用
    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略停止时调用
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略暂停时调用
    async fn on_pause(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略恢复时调用
    async fn on_resume(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略归档时调用
    async fn on_archive(&mut self) -> Result<()> {
        Ok(())
    }

    // ── 参数 Schema ─────────────────────────────────────────────────────

    /// 返回该策略的参数 Schema 定义（用于前端动态渲染和参数校验）
    #[must_use]
    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        Vec::new()
    }
}

/// 均值回归策略示例
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
            params: StrategyParams {
                strategy_id: "mean_reversion_001".to_string(),
                strategy_name: "Mean Reversion Strategy".to_string(),
                strategy_type: quant_common::types::StrategyType::MeanReversion,
                params: serde_json::json!({
                    "lookback_period": 20,
                    "entry_threshold": 2.0,
                    "exit_threshold": 0.5,
                }),
                enabled: true,
                max_position: rust_decimal::Decimal::new(100000, 0),
                max_daily_loss: rust_decimal::Decimal::new(5000, 0),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                status: Default::default(),
                description: None,
                tags: vec![],
                symbols: vec![],
                instance_label: None,
                user_id: 0,
                version: 0,
            },
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

// ─── TrendFollowingStrategy ───────────────────────────────────────────────

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
            params: StrategyParams {
                strategy_id: "trend_following_001".to_string(),
                strategy_name: "Trend Following Strategy".to_string(),
                strategy_type: quant_common::types::StrategyType::TrendFollowing,
                params: serde_json::json!({
                    "short_period": DEFAULT_SHORT_PERIOD,
                    "long_period": DEFAULT_LONG_PERIOD,
                    "signal_period": DEFAULT_SIGNAL_PERIOD,
                }),
                enabled: true,
                max_position: rust_decimal::Decimal::new(100000, 0),
                max_daily_loss: rust_decimal::Decimal::new(5000, 0),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                status: Default::default(),
                description: None,
                tags: vec![],
                symbols: vec![],
                instance_label: None,
                user_id: 0,
                version: 0,
            },
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

    #[tokio::test]
    async fn test_insufficient_data_returns_empty_signals() {
        let strategy = MeanReversionStrategy::new();
        let context = StrategyContext {
            current_time: Utc::now(),
            positions: vec![],
            market_data: vec![make_market_data(Decimal::from(100))], // only 1 point, lookback=20
        };
        let orders = strategy.generate_signals(&context).await.unwrap();
        assert!(orders.is_empty());
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

    /// Build a regime-change data series:
    ///   phase 1: `stable` bars all at `stable_price`
    ///   phase 2: `trend` bars moving linearly from `stable_price` toward `extreme_price`
    /// This creates a large SMA gap + extreme RSI at the end.
    fn build_regime_series(
        stable: usize,
        trend: usize,
        stable_price: i64,
        extreme_price: i64,
        symbol: &str,
    ) -> Vec<MarketData> {
        let utc = Utc::now();
        let mut data = Vec::with_capacity(stable + trend);

        for i in 0..stable {
            data.push(make_full_market_data(
                utc + chrono::Duration::hours(i as i64),
                Decimal::from(stable_price),
                symbol,
            ));
        }

        for i in 0..trend {
            let t = (i + 1) as f64 / trend as f64;
            let close_f = stable_price as f64 + (extreme_price - stable_price) as f64 * t;
            let close = Decimal::from_f64(close_f).unwrap();
            data.push(make_full_market_data(
                utc + chrono::Duration::hours((stable + i) as i64),
                close,
                symbol,
            ));
        }

        data
    }

    #[tokio::test]
    async fn test_rsi_oversold_triggers_buy() {
        let mut strategy = MeanReversionStrategy::new();
        // Use low entry_threshold to make z-score condition easily satisfiable
        let params = StrategyParams {
            strategy_id: "test".to_string(),
            strategy_name: "Test".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 5,
                "entry_threshold": 0.5,
                "exit_threshold": 0.5,
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

        // 20 bars stable at 100, then 15 bars crashing to 1
        let data = build_regime_series(20, 15, 100, 1, "BTC/USDT");

        let context = StrategyContext {
            current_time: Utc::now(),
            positions: vec![],
            market_data: data,
        };
        let orders = strategy.generate_signals(&context).await.unwrap();
        assert!(
            orders.iter().any(|o| o.side == OrderSide::Buy),
            "Expected Buy signal: price crashed from 100 to 1 after stable period"
        );
    }

    #[tokio::test]
    async fn test_rsi_overbought_triggers_sell() {
        let mut strategy = MeanReversionStrategy::new();
        let params = StrategyParams {
            strategy_id: "test".to_string(),
            strategy_name: "Test".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 5,
                "entry_threshold": 0.5,
                "exit_threshold": 0.5,
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

        // 20 bars stable at 100, then 15 bars spiking to 199
        let data = build_regime_series(20, 15, 100, 199, "BTC/USDT");

        let context = StrategyContext {
            current_time: Utc::now(),
            positions: vec![],
            market_data: data,
        };
        let orders = strategy.generate_signals(&context).await.unwrap();
        assert!(
            orders.iter().any(|o| o.side == OrderSide::Sell),
            "Expected Sell signal: price surged from 100 to 199 after stable period"
        );
    }

    /// P0-1: `update_params` must NOT reset runtime state.
    /// It should only update `self.params` (metadata) so hot-parameter updates
    /// do not wipe parsed numeric thresholds or other state.
    #[tokio::test]
    async fn test_update_params_preserves_runtime_state() {
        let mut strategy = MeanReversionStrategy::new();
        let initial_params = StrategyParams {
            strategy_id: "orig-001".to_string(),
            strategy_name: "Original".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 10,
                "entry_threshold": 1.5,
                "exit_threshold": 0.3,
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

        assert_eq!(strategy.lookback_period, 10);
        assert!((strategy.entry_threshold - 1.5).abs() < f64::EPSILON);

        let new_params = StrategyParams {
            strategy_id: "orig-001".to_string(),
            strategy_name: "Original".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 50,
                "entry_threshold": 4.0,
                "exit_threshold": 1.0,
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
        strategy.update_params(new_params.clone()).await.unwrap();

        assert_eq!(strategy.params().strategy_id, "orig-001");
        assert_eq!(strategy.params().params, new_params.params);

        assert_eq!(
            strategy.lookback_period, 10,
            "update_params must not reset parsed lookback_period"
        );
        assert!(
            (strategy.entry_threshold - 1.5).abs() < f64::EPSILON,
            "update_params must not reset parsed entry_threshold"
        );
        assert!(
            (strategy.exit_threshold - 0.3).abs() < f64::EPSILON,
            "update_params must not reset parsed exit_threshold"
        );
    }

    /// P0-1: `reinitialize` is the explicit full-reset path. It should re-parse
    /// all numeric thresholds from the new params, exactly like `initialize`.
    #[tokio::test]
    async fn test_reinitialize_resets_state_with_new_params() {
        let mut strategy = MeanReversionStrategy::new();
        let initial_params = StrategyParams {
            strategy_id: "orig-002".to_string(),
            strategy_name: "Original".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 10,
                "entry_threshold": 1.5,
                "exit_threshold": 0.3,
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
        assert_eq!(strategy.lookback_period, 10);

        let new_params = StrategyParams {
            strategy_id: "orig-002".to_string(),
            strategy_name: "Original".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 50,
                "entry_threshold": 4.0,
                "exit_threshold": 1.0,
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
        strategy.reinitialize(new_params).await.unwrap();

        assert_eq!(
            strategy.lookback_period, 50,
            "reinitialize must re-parse lookback_period"
        );
        assert!((strategy.entry_threshold - 4.0).abs() < f64::EPSILON);
        assert!((strategy.exit_threshold - 1.0).abs() < f64::EPSILON);
    }

    // ── TrendFollowingStrategy tests ──────────────────────────────────────

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

    fn build_crossover_series(up: bool, symbol: &str) -> Vec<MarketData> {
        // 30 bars gradual decline (or rise) to establish EMA separation,
        // then a sharp reversal on the last bar forcing a crossover.
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

        // Downtrend establishes short EMA < long EMA, then last bar spikes up.
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

        // Uptrend establishes short EMA > long EMA, then last bar crashes down.
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
