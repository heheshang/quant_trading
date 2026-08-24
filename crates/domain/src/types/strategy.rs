use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ─── Strategy Lifecycle Status ──────────────────────────────────────────────

/// Strategy lifecycle status.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyStatus {
    #[default]
    Draft,
    Backtesting,
    Deployed,
    Running,
    Paused,
    Archived,
}

impl std::str::FromStr for StrategyStatus {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "Backtesting" => Ok(Self::Backtesting),
            "Deployed" => Ok(Self::Deployed),
            "Running" => Ok(Self::Running),
            "Paused" => Ok(Self::Paused),
            "Archived" => Ok(Self::Archived),
            _ => Ok(Self::Draft),
        }
    }
}

impl StrategyStatus {
    /// Check whether a transition from `self` to `to` is allowed.
    #[must_use]
    pub fn can_transition_to(&self, to: StrategyStatus) -> bool {
        matches!(
            (*self, to),
            (
                Self::Draft,
                Self::Backtesting | Self::Deployed | Self::Archived
            ) | (Self::Backtesting, Self::Deployed | Self::Draft)
                | (Self::Deployed, Self::Running | Self::Draft)
                | (Self::Running, Self::Paused | Self::Archived)
                | (Self::Paused, Self::Running | Self::Archived)
        )
    }
}

/// Guard predicate for status transitions.
pub type StrategyGuard = Box<dyn Fn(&StrategyParams) -> bool + Send + Sync>;

/// A permitted status transition with an optional guard predicate.
pub struct StatusTransition {
    pub from: StrategyStatus,
    pub to: StrategyStatus,
    pub guard: Option<StrategyGuard>,
}

impl std::fmt::Debug for StatusTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusTransition")
            .field("from", &self.from)
            .field("to", &self.to)
            .finish()
    }
}

/// All allowed status transitions (without guards).
#[must_use]
pub fn allowed_transitions() -> Vec<StatusTransition> {
    use StrategyStatus::*;
    vec![
        StatusTransition {
            from: Draft,
            to: Backtesting,
            guard: None,
        },
        StatusTransition {
            from: Draft,
            to: Deployed,
            guard: None,
        },
        StatusTransition {
            from: Draft,
            to: Archived,
            guard: None,
        },
        StatusTransition {
            from: Backtesting,
            to: Deployed,
            guard: None,
        },
        StatusTransition {
            from: Backtesting,
            to: Draft,
            guard: None,
        },
        StatusTransition {
            from: Deployed,
            to: Running,
            guard: None,
        },
        StatusTransition {
            from: Deployed,
            to: Draft,
            guard: None,
        },
        StatusTransition {
            from: Running,
            to: Paused,
            guard: None,
        },
        StatusTransition {
            from: Running,
            to: Archived,
            guard: None,
        },
        StatusTransition {
            from: Paused,
            to: Running,
            guard: None,
        },
        StatusTransition {
            from: Paused,
            to: Archived,
            guard: None,
        },
    ]
}

/// Information about a running scheduler task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerTaskInfo {
    pub strategy_id: String,
    pub strategy_name: String,
    pub status: StrategyStatus,
    pub interval_secs: u64,
    pub last_run_at: Option<DateTime<Utc>>,
    pub error_count: u32,
}

// ─── Strategy ────────────────────────────────────────────────────────────────

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
    /// E.g., "My Trend Bot v2" — used for display when multiple strategies share a type.
    #[serde(default)]
    pub instance_label: Option<String>,
    /// 策略所有者用户 ID (可为空，与策略管理平台的模式保持一致)
    #[serde(default)]
    pub user_id: i64,
    /// Optimistic lock version. Incremented on each update.
    /// Used for concurrent-safe modifications in multi-session scenarios.
    #[serde(default)]
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrategyType {
    TrendFollowing,
    MeanReversion,
    Macd,
    Rsi,
}

impl StrategyType {
    /// Convert a type name string (e.g., "MeanReversion") to the enum variant.
    /// Uses the Debug representation for matching.
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
    ///
    /// # Errors
    ///
    /// Returns `StrategyError::InvalidTransition` if the transition is not permitted.
    pub fn transition_to(
        &mut self,
        target: StrategyStatus,
    ) -> Result<StrategyStatus, StrategyError> {
        if !self.status.can_transition_to(target) {
            return Err(StrategyError::InvalidTransition {
                from: self.status,
                to: target,
            });
        }
        self.status = target;
        self.updated_at = Utc::now();
        Ok(self.status)
    }
}

impl StrategyParams {
    /// 创建 StrategyParams 的 Builder（Builder 模式），减少多字段构造样板。
    #[must_use]
    pub fn builder(
        strategy_id: impl Into<String>,
        strategy_name: impl Into<String>,
        strategy_type: StrategyType,
    ) -> StrategyParamsBuilder {
        StrategyParamsBuilder::new(strategy_id, strategy_name, strategy_type)
    }
}

/// StrategyParams 的 Builder（Builder 模式）。
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
    /// 从必填身份字段开始构建。
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

    /// 设置策略参数（JSON）。
    #[must_use]
    pub fn params(mut self, v: serde_json::Value) -> Self {
        self.params = v;
        self
    }

    /// 设置启用状态。
    #[must_use]
    pub fn enabled(mut self, v: bool) -> Self {
        self.enabled = v;
        self
    }

    /// 设置最大仓位。
    #[must_use]
    pub fn max_position(mut self, v: Decimal) -> Self {
        self.max_position = v;
        self
    }

    /// 设置单日最大亏损。
    #[must_use]
    pub fn max_daily_loss(mut self, v: Decimal) -> Self {
        self.max_daily_loss = v;
        self
    }

    /// 设置生命周期状态。
    #[must_use]
    pub fn status(mut self, v: StrategyStatus) -> Self {
        self.status = v;
        self
    }

    /// 设置描述。
    #[must_use]
    pub fn description(mut self, v: Option<String>) -> Self {
        self.description = v;
        self
    }

    /// 设置标签。
    #[must_use]
    pub fn tags(mut self, v: Vec<String>) -> Self {
        self.tags = v;
        self
    }

    /// 设置交易对。
    #[must_use]
    pub fn symbols(mut self, v: Vec<String>) -> Self {
        self.symbols = v;
        self
    }

    /// 设置实例标签。
    #[must_use]
    pub fn instance_label(mut self, v: Option<String>) -> Self {
        self.instance_label = v;
        self
    }

    /// 设置所有者用户 ID。
    #[must_use]
    pub fn user_id(mut self, v: i64) -> Self {
        self.user_id = v;
        self
    }

    /// 设置乐观锁版本号。
    #[must_use]
    pub fn version(mut self, v: i64) -> Self {
        self.version = v;
        self
    }

    /// 构建 StrategyParams，created_at / updated_at 默认取当前时间。
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

/// Strategy lifecycle error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StrategyError {
    #[error("Invalid status transition: {from:?} → {to:?} is not allowed")]
    InvalidTransition {
        from: StrategyStatus,
        to: StrategyStatus,
    },
}

// ─── Backtest Result ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub id: Option<i64>,
    pub strategy_id: String,
    #[serde(default)]
    pub strategy_name: Option<String>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub final_capital: Decimal,
    pub total_return: Decimal,
    pub annual_return: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub profit_loss_ratio: Decimal,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
    pub equity_curve: Vec<(DateTime<Utc>, Decimal)>,
    #[serde(default)]
    pub trades: Vec<BacktestTrade>,
}
/// A single executed trade recorded by the backtest engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTrade {
    pub date: DateTime<Utc>,
    pub symbol: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub amount: Decimal,
    pub commission: Decimal,
}

impl BacktestResult {
    /// Profit/Loss ratio (revenue / cost).
    pub fn profit_factor(&self) -> Decimal {
        let losses = self.total_trades - self.winning_trades;
        if losses == 0 {
            return if self.winning_trades > 0 {
                Decimal::MAX
            } else {
                Decimal::ZERO
            };
        }
        Decimal::from(self.winning_trades) / Decimal::from(losses)
    }

    /// Net profit in absolute terms.
    pub fn net_profit(&self) -> Decimal {
        self.final_capital - self.initial_capital
    }

    /// Duration of the backtest in days.
    pub fn duration_days(&self) -> i64 {
        (self.end_date - self.start_date).num_days()
    }
}

// ─── Risk Metrics ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub timestamp: DateTime<Utc>,
    pub var_95: Decimal,
    pub var_99: Decimal,
    pub portfolio_volatility: Decimal,
    pub beta: Decimal,
    pub concentration_risk: Decimal,
    pub leverage: Decimal,
}

impl RiskMetrics {
    /// Ratio of VaR(99) to VaR(95) — indicates tail risk.
    pub fn tail_ratio(&self) -> Decimal {
        if self.var_95.is_zero() {
            return Decimal::ZERO;
        }
        self.var_99 / self.var_95
    }

    /// True if leverage exceeds a given threshold.
    #[must_use]
    pub fn is_over_leveraged(&self, max_leverage: Decimal) -> bool {
        self.leverage > max_leverage
    }
}

// ─── Alerts ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: i64,
    pub level: AlertLevel,
    pub source: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

impl Alert {
    /// Create a new unacknowledged alert (alert_id = 0, assigned by DB on insert).
    pub fn new(level: AlertLevel, source: String, message: String) -> Self {
        Self {
            alert_id: 0,
            level,
            source,
            message,
            timestamp: Utc::now(),
            acknowledged: false,
        }
    }

    /// True if critical severity.
    #[must_use]
    pub fn is_critical(&self) -> bool {
        self.level == AlertLevel::Critical
    }

    /// Age of the alert in seconds.
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.timestamp).num_seconds()
    }

    /// Mark as acknowledged.
    pub fn acknowledge(&mut self) {
        self.acknowledged = true;
    }
}
