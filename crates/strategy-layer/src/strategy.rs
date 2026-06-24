use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_common::types::{MarketData, Order, Position, StrategyParams};
use quant_common::Result;

const DEFAULT_ENTRY_THRESHOLD: f64 = 2.0;

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

    /// 更新策略参数
    async fn update_params(&mut self, params: StrategyParams) -> Result<()>;
}

/// 均值回归策略示例
pub struct MeanReversionStrategy {
    params: StrategyParams,
    lookback_period: usize,
    entry_threshold: f64,
    exit_threshold: f64,
}

impl MeanReversionStrategy {
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
            },
            lookback_period: 20,
            entry_threshold: DEFAULT_ENTRY_THRESHOLD,
            exit_threshold: 0.5,
        }
    }
}

#[async_trait]
impl Strategy for MeanReversionStrategy {
    async fn initialize(&mut self, params: StrategyParams) -> Result<()> {
        self.params = params.clone();

        // 从参数中提取配置
        if let Some(lookback) = params.params.get("lookback_period") {
            self.lookback_period = lookback.as_u64().unwrap_or(20) as usize;
        }
        if let Some(entry) = params.params.get("entry_threshold") {
            self.entry_threshold = entry.as_f64().unwrap_or(DEFAULT_ENTRY_THRESHOLD);
        }
        if let Some(exit) = params.params.get("exit_threshold") {
            self.exit_threshold = exit.as_f64().unwrap_or(0.5);
        }

        Ok(())
    }

    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>> {
        let orders = Vec::new();

        // 示例逻辑：计算价格偏离均值的程度
        if context.market_data.len() < self.lookback_period {
            return Ok(orders);
        }

        // 实际策略实现会更复杂
        // 这里仅作演示

        Ok(orders)
    }

    fn name(&self) -> &str {
        &self.params.strategy_name
    }

    fn params(&self) -> &StrategyParams {
        &self.params
    }

    async fn update_params(&mut self, params: StrategyParams) -> Result<()> {
        self.initialize(params).await
    }
}

impl Default for MeanReversionStrategy {
    fn default() -> Self {
        Self::new()
    }
}
