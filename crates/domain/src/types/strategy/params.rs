//! 策略参数模块
//!
//! 定义策略参数和 Builder 模式。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::types::strategy::status::StrategyStatus;

/// 策略参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParams {
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: StrategyType,
    pub params: serde_json::Value,
    pub enabled: bool,
    pub max_position: Decimal,
    pub max_daily_loss: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Current lifecycle status.
    #[serde(default)]
    pub status: StrategyStatus,
    /// Human-readable description of the strategy purpose.
    #[serde(default)]
    pub description: Option<String>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Trading symbols this strategy operates on.
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Human-friendly label for distinguishing multi-instance strategies of the same type.
    #[serde(default)]
    pub instance_label: Option<String>,
    /// 策略所有者用户 ID
    #[serde(default)]
    pub user_id: i64,
    /// Optimistic lock version. Incremented on each update.
    #[serde(default)]
    pub version: i64,
}

/// 策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrategyType {
    TrendFollowing,
    MeanReversion,
    Macd,
    Rsi,
}

impl StrategyType {
    /// Convert a type name string to the enum variant.
    pub fn from_type_name(name: &str) -> Option<Self> {
        use StrategyType::*;
        match name {
            "TrendFollowing" => Some(TrendFollowing),
            "MeanReversion" => Some(MeanReversion),
            "MACD" | "Macd" => Some(Macd),
            "RSI" | "Rsi" => Some(Rsi),
            _ => None,
        }
    }
}

impl StrategyParams {
    /// Basic validation — strategy must have a name and valid type.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.strategy_name.is_empty() && self.max_position > Decimal::ZERO
    }

    /// Enable the strategy.
    pub fn enable(&mut self) {
        self.enabled = true;
        self.updated_at = Utc::now();
    }

    /// Disable the strategy.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.updated_at = Utc::now();
    }

    /// Transition to a new status if allowed.
    pub fn transition_to(
        &mut self,
        target: StrategyStatus,
    ) -> Result<StrategyStatus, super::error::StrategyError> {
        if !self.status.can_transition_to(target) {
            return Err(super::error::StrategyError::InvalidTransition {
                from: self.status,
                to: target,
            });
        }
        self.status = target;
        self.updated_at = Utc::now();
        Ok(self.status)
    }

    /// 创建 StrategyParams 的 Builder（Builder 模式）
    #[must_use]
    pub fn builder(
        strategy_id: impl Into<String>,
        strategy_name: impl Into<String>,
        strategy_type: StrategyType,
    ) -> StrategyParamsBuilder {
        StrategyParamsBuilder::new(strategy_id, strategy_name, strategy_type)
    }
}

/// StrategyParams 的 Builder
#[derive(Debug)]
pub struct StrategyParamsBuilder {
    strategy_id: String,
    strategy_name: String,
    strategy_type: StrategyType,
    params: serde_json::Value,
    enabled: bool,
    max_position: Decimal,
    max_daily_loss: Decimal,
    status: StrategyStatus,
    description: Option<String>,
    tags: Vec<String>,
    symbols: Vec<String>,
    instance_label: Option<String>,
    user_id: i64,
    version: i64,
}

impl StrategyParamsBuilder {
    /// 从必填身份字段开始构建
    #[must_use]
    pub fn new(
        strategy_id: impl Into<String>,
        strategy_name: impl Into<String>,
        strategy_type: StrategyType,
    ) -> Self {
        Self {
            strategy_id: strategy_id.into(),
            strategy_name: strategy_name.into(),
            strategy_type,
            params: serde_json::json!({}),
            enabled: true,
            max_position: Decimal::ZERO,
            max_daily_loss: Decimal::ZERO,
            status: StrategyStatus::Draft,
            description: None,
            tags: Vec::new(),
            symbols: Vec::new(),
            instance_label: None,
            user_id: 0,
            version: 0,
        }
    }

    /// 设置策略参数（JSON）
    #[must_use]
    pub fn params(mut self, v: serde_json::Value) -> Self {
        self.params = v;
        self
    }

    /// 设置启用状态
    #[must_use]
    pub fn enabled(mut self, v: bool) -> Self {
        self.enabled = v;
        self
    }

    /// 设置最大仓位
    #[must_use]
    pub fn max_position(mut self, v: Decimal) -> Self {
        self.max_position = v;
        self
    }

    /// 设置单日最大亏损
    #[must_use]
    pub fn max_daily_loss(mut self, v: Decimal) -> Self {
        self.max_daily_loss = v;
        self
    }

    /// 设置生命周期状态
    #[must_use]
    pub fn status(mut self, v: StrategyStatus) -> Self {
        self.status = v;
        self
    }

    /// 设置描述
    #[must_use]
    pub fn description(mut self, v: Option<String>) -> Self {
        self.description = v;
        self
    }

    /// 设置标签
    #[must_use]
    pub fn tags(mut self, v: Vec<String>) -> Self {
        self.tags = v;
        self
    }

    /// 设置交易对
    #[must_use]
    pub fn symbols(mut self, v: Vec<String>) -> Self {
        self.symbols = v;
        self
    }

    /// 设置实例标签
    #[must_use]
    pub fn instance_label(mut self, v: Option<String>) -> Self {
        self.instance_label = v;
        self
    }

    /// 设置所有者用户 ID
    #[must_use]
    pub fn user_id(mut self, v: i64) -> Self {
        self.user_id = v;
        self
    }

    /// 设置乐观锁版本号
    #[must_use]
    pub fn version(mut self, v: i64) -> Self {
        self.version = v;
        self
    }

    /// 构建 StrategyParams，created_at / updated_at 默认取当前时间
    #[must_use]
    pub fn build(self) -> StrategyParams {
        let now = Utc::now();
        StrategyParams {
            strategy_id: self.strategy_id,
            strategy_name: self.strategy_name,
            strategy_type: self.strategy_type,
            params: self.params,
            enabled: self.enabled,
            max_position: self.max_position,
            max_daily_loss: self.max_daily_loss,
            created_at: now,
            updated_at: now,
            status: self.status,
            description: self.description,
            tags: self.tags,
            symbols: self.symbols,
            instance_label: self.instance_label,
            user_id: self.user_id,
            version: self.version,
        }
    }
}
