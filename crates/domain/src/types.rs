use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ─── Instrument ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub symbol: String,
    pub exchange: Exchange,
    pub instrument_type: InstrumentType,
    pub contract_multiplier: Decimal,
    pub tick_size: Decimal,
    pub lot_size: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Exchange {
    SSE,
    SZSE,
    CFFEX,
    SHFE,
    DCE,
    CZCE,
    INE,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstrumentType {
    Stock,
    Future,
    Option,
    ETF,
    Index,
    Bond,
}

// ─── Market Data ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub turnover: Decimal,
    pub open_interest: Option<Decimal>,
    pub bid_prices: Vec<Decimal>,
    pub bid_volumes: Vec<Decimal>,
    pub ask_prices: Vec<Decimal>,
    pub ask_volumes: Vec<Decimal>,
}

impl MarketData {
    /// Best bid price (highest buy order).
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bid_prices.first().copied()
    }

    /// Best ask price (lowest sell order).
    pub fn best_ask(&self) -> Option<Decimal> {
        self.ask_prices.first().copied()
    }

    /// Bid-ask spread as absolute value.
    pub fn spread(&self) -> Option<Decimal> {
        Some((self.best_ask()? - self.best_bid()?).abs())
    }

    /// Mid price between best bid and ask.
    pub fn mid_price(&self) -> Option<Decimal> {
        Some((self.best_bid()? + self.best_ask()?) / Decimal::TWO)
    }

    /// Price change from open to close.
    pub fn price_change(&self) -> Decimal {
        self.close - self.open
    }

    /// True if close > open (bullish candle).
    #[must_use]
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }
}

// ─── Order Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    StopLimit,
    TWAP,
    VWAP,
    Iceberg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,
    Submitted,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
    Expired,
}

impl OrderStatus {
    /// True if the order is in a terminal (non-modifiable) state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Filled | Self::Cancelled | Self::Rejected | Self::Expired
        )
    }

    /// True if the order can still be modified or cancelled.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Pending | Self::Submitted | Self::PartiallyFilled
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: i64,
    pub strategy_id: String,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub commission: Decimal,
    pub slippage: Decimal,
}

impl Order {
    /// Ratio of filled to total quantity (0.0 — 1.0).
    pub fn fill_ratio(&self) -> Decimal {
        if self.quantity.is_zero() {
            return Decimal::ZERO;
        }
        self.filled_quantity / self.quantity
    }

    /// Remaining unfilled quantity.
    pub fn remaining_quantity(&self) -> Decimal {
        (self.quantity - self.filled_quantity).max(Decimal::ZERO)
    }

    /// Whether this order can be cancelled.
    #[must_use]
    pub fn can_cancel(&self) -> bool {
        self.status.is_active()
    }

    /// Estimated total value of the order at its limit price.
    /// Returns None for market orders (no price).
    pub fn estimated_value(&self) -> Option<Decimal> {
        self.price.map(|p| p * self.quantity)
    }

    /// Estimated total value at a given (market) price.
    pub fn estimated_value_at(&self, price: Decimal) -> Decimal {
        price * self.quantity
    }

    /// Create a new pending order (order_id = 0, assigned by DB on insert).
    pub fn new(
        strategy_id: String,
        symbol: String,
        order_type: OrderType,
        side: OrderSide,
        price: Option<Decimal>,
        quantity: Decimal,
    ) -> Self {
        Self {
            order_id: 0,
            strategy_id,
            symbol,
            order_type,
            side,
            price,
            quantity,
            filled_quantity: Decimal::ZERO,
            status: OrderStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: Decimal::ZERO,
            slippage: Decimal::ZERO,
        }
    }
}

// ─── Position ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub available_quantity: Decimal,
    pub avg_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl Position {
    /// Unrealized PnL as a percentage of cost basis.
    pub fn pnl_percentage(&self) -> Decimal {
        let cost_basis = self.avg_price * self.quantity;
        if cost_basis.is_zero() {
            return Decimal::ZERO;
        }
        self.unrealized_pnl / cost_basis
    }

    /// Frozen quantity (total - available).
    pub fn frozen_quantity(&self) -> Decimal {
        (self.quantity - self.available_quantity).max(Decimal::ZERO)
    }

    /// Total PnL (realized + unrealized).
    pub fn total_pnl(&self) -> Decimal {
        self.realized_pnl + self.unrealized_pnl
    }

    /// True if position has any quantity.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.quantity > Decimal::ZERO
    }
}

// ─── Account ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub account_id: i64,
    pub total_assets: Decimal,
    pub available_cash: Decimal,
    pub frozen_cash: Decimal,
    pub market_value: Decimal,
    pub total_pnl: Decimal,
    pub daily_pnl: Decimal,
    pub margin: Decimal,
    pub margin_ratio: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl Account {
    /// Cash + market value (should equal total_assets).
    pub fn total_equity(&self) -> Decimal {
        self.available_cash + self.frozen_cash + self.market_value
    }

    /// Available margin for new positions.
    pub fn available_margin(&self) -> Decimal {
        (self.total_assets - self.margin).max(Decimal::ZERO)
    }

    /// Margin usage as a ratio (0.0 — 1.0).
    pub fn margin_usage_ratio(&self) -> Decimal {
        if self.total_assets.is_zero() {
            return Decimal::ZERO;
        }
        (self.margin / self.total_assets).min(Decimal::ONE)
    }

    /// True if account has sufficient cash for a trade of given value.
    #[must_use]
    pub fn can_cover(&self, required_cash: Decimal) -> bool {
        self.available_cash >= required_cash
    }
}

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
    Arbitrage,
    MarketMaking,
    Statistical,
    MachineLearning,
    Custom,
}

impl StrategyType {
    /// Convert a type name string (e.g., "MeanReversion") to the enum variant.
    /// Uses the Debug representation for matching.
    pub fn from_type_name(name: &str) -> Option<Self> {
        use StrategyType::*;
        match name {
            "TrendFollowing" => Some(TrendFollowing),
            "MeanReversion" => Some(MeanReversion),
            "Arbitrage" => Some(Arbitrage),
            "MarketMaking" => Some(MarketMaking),
            "Statistical" => Some(Statistical),
            "MachineLearning" => Some(MachineLearning),
            "Custom" => Some(Custom),
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_market_data() -> MarketData {
        MarketData {
            symbol: "000001.SZ".into(),
            timestamp: Utc::now(),
            open: dec!(10.00),
            high: dec!(10.50),
            low: dec!(9.80),
            close: dec!(10.20),
            volume: dec!(1000000),
            turnover: dec!(10200000),
            open_interest: Some(dec!(50000)),
            bid_prices: vec![dec!(10.15), dec!(10.10), dec!(10.05)],
            bid_volumes: vec![dec!(1000), dec!(2000), dec!(1500)],
            ask_prices: vec![dec!(10.25), dec!(10.30), dec!(10.35)],
            ask_volumes: vec![dec!(800), dec!(1200), dec!(2000)],
        }
    }

    fn make_order() -> Order {
        Order {
            order_id: 0,
            strategy_id: "test_strategy".into(),
            symbol: "000001.SZ".into(),
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: Some(dec!(10.00)),
            quantity: dec!(1000),
            filled_quantity: dec!(300),
            status: OrderStatus::PartiallyFilled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            commission: dec!(5.0),
            slippage: dec!(0.01),
        }
    }

    // ── MarketData ────────────────────────────────────────────────────────────

    #[test]
    fn test_marketdata_best_bid_with_orders() {
        let md = make_market_data();
        assert_eq!(md.best_bid(), Some(dec!(10.15)));
    }

    #[test]
    fn test_marketdata_best_bid_empty() {
        let md = MarketData {
            bid_prices: vec![],
            ..make_market_data()
        };
        assert_eq!(md.best_bid(), None);
    }

    #[test]
    fn test_marketdata_best_ask_with_orders() {
        let md = make_market_data();
        assert_eq!(md.best_ask(), Some(dec!(10.25)));
    }

    #[test]
    fn test_marketdata_best_ask_empty() {
        let md = MarketData {
            ask_prices: vec![],
            ..make_market_data()
        };
        assert_eq!(md.best_ask(), None);
    }

    #[test]
    fn test_marketdata_spread_with_orders() {
        let md = make_market_data();
        assert_eq!(md.spread(), Some(dec!(0.10)));
    }

    #[test]
    fn test_marketdata_spread_no_bid() {
        let md = MarketData {
            bid_prices: vec![],
            ..make_market_data()
        };
        assert_eq!(md.spread(), None);
    }

    #[test]
    fn test_marketdata_spread_no_ask() {
        let md = MarketData {
            ask_prices: vec![],
            ..make_market_data()
        };
        assert_eq!(md.spread(), None);
    }

    #[test]
    fn test_marketdata_spread_crossed_book() {
        let md = MarketData {
            bid_prices: vec![dec!(10.25)],
            ask_prices: vec![dec!(10.15)],
            ..make_market_data()
        };
        // bid > ask is a crossed book — spread should be the absolute difference
        assert_eq!(md.spread(), Some(dec!(0.10)));
    }

    #[test]
    fn test_marketdata_mid_price_normal() {
        let md = make_market_data();
        assert_eq!(md.mid_price(), Some(dec!(10.20)));
    }

    #[test]
    fn test_marketdata_mid_price_no_bid() {
        let md = MarketData {
            bid_prices: vec![],
            ..make_market_data()
        };
        assert_eq!(md.mid_price(), None);
    }

    #[test]
    fn test_marketdata_price_change_positive() {
        let md = make_market_data();
        assert_eq!(md.price_change(), dec!(0.20));
    }

    #[test]
    fn test_marketdata_price_change_negative() {
        let md = MarketData {
            open: dec!(10.50),
            close: dec!(10.20),
            ..make_market_data()
        };
        assert_eq!(md.price_change(), dec!(-0.30));
    }

    #[test]
    fn test_marketdata_price_change_zero() {
        let md = MarketData {
            open: dec!(10.00),
            close: dec!(10.00),
            ..make_market_data()
        };
        assert_eq!(md.price_change(), dec!(0.00));
    }

    #[test]
    fn test_marketdata_is_bullish_true() {
        let md = make_market_data();
        assert!(md.is_bullish());
    }

    #[test]
    fn test_marketdata_is_bullish_false() {
        let md = MarketData {
            open: dec!(10.50),
            close: dec!(10.20),
            ..make_market_data()
        };
        assert!(!md.is_bullish());
    }

    #[test]
    fn test_marketdata_is_bullish_equal() {
        let md = MarketData {
            open: dec!(10.00),
            close: dec!(10.00),
            ..make_market_data()
        };
        assert!(!md.is_bullish());
    }

    // ── OrderStatus ───────────────────────────────────────────────────────────

    #[test]
    fn test_orderstatus_is_terminal_filled() {
        assert!(OrderStatus::Filled.is_terminal());
    }

    #[test]
    fn test_orderstatus_is_terminal_cancelled() {
        assert!(OrderStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_orderstatus_is_terminal_rejected() {
        assert!(OrderStatus::Rejected.is_terminal());
    }

    #[test]
    fn test_orderstatus_is_terminal_expired() {
        assert!(OrderStatus::Expired.is_terminal());
    }

    #[test]
    fn test_orderstatus_is_terminal_pending() {
        assert!(!OrderStatus::Pending.is_terminal());
    }

    #[test]
    fn test_orderstatus_is_terminal_submitted() {
        assert!(!OrderStatus::Submitted.is_terminal());
    }

    #[test]
    fn test_orderstatus_is_terminal_partially_filled() {
        assert!(!OrderStatus::PartiallyFilled.is_terminal());
    }

    #[test]
    fn test_orderstatus_is_active_pending() {
        assert!(OrderStatus::Pending.is_active());
    }

    #[test]
    fn test_orderstatus_is_active_submitted() {
        assert!(OrderStatus::Submitted.is_active());
    }

    #[test]
    fn test_orderstatus_is_active_partially_filled() {
        assert!(OrderStatus::PartiallyFilled.is_active());
    }

    #[test]
    fn test_orderstatus_is_active_filled() {
        assert!(!OrderStatus::Filled.is_active());
    }

    #[test]
    fn test_orderstatus_is_active_cancelled() {
        assert!(!OrderStatus::Cancelled.is_active());
    }

    #[test]
    fn test_orderstatus_is_active_rejected() {
        assert!(!OrderStatus::Rejected.is_active());
    }

    #[test]
    fn test_orderstatus_is_active_expired() {
        assert!(!OrderStatus::Expired.is_active());
    }

    // ── Order ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_order_new_creates_pending_order() {
        let order = Order::new(
            "strat_1".into(),
            "000001.SZ".into(),
            OrderType::Limit,
            OrderSide::Buy,
            Some(dec!(10.00)),
            dec!(500),
        );
        assert_eq!(order.strategy_id, "strat_1");
        assert_eq!(order.symbol, "000001.SZ");
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.price, Some(dec!(10.00)));
        assert_eq!(order.quantity, dec!(500));
        assert_eq!(order.filled_quantity, dec!(0));
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(order.commission, dec!(0));
        assert_eq!(order.slippage, dec!(0));
    }

    #[test]
    fn test_order_new_has_zero_id() {
        // New orders use 0 as placeholder — DB assigns BIGSERIAL on INSERT.
        let o1 = Order::new(
            "s".into(),
            "sym".into(),
            OrderType::Market,
            OrderSide::Sell,
            None,
            dec!(100),
        );
        assert_eq!(o1.order_id, 0);
    }

    #[test]
    fn test_order_fill_ratio_partial() {
        let order = make_order();
        assert_eq!(order.fill_ratio(), dec!(0.3));
    }

    #[test]
    fn test_order_fill_ratio_fully_filled() {
        let order = Order {
            filled_quantity: dec!(1000),
            ..make_order()
        };
        assert_eq!(order.fill_ratio(), dec!(1.0));
    }

    #[test]
    fn test_order_fill_ratio_zero_quantity() {
        let order = Order {
            quantity: dec!(0),
            filled_quantity: dec!(0),
            ..make_order()
        };
        assert_eq!(order.fill_ratio(), dec!(0));
    }

    #[test]
    fn test_order_fill_ratio_no_fill() {
        let order = Order {
            filled_quantity: dec!(0),
            ..make_order()
        };
        assert_eq!(order.fill_ratio(), dec!(0));
    }

    #[test]
    fn test_order_remaining_quantity_partial() {
        let order = make_order();
        assert_eq!(order.remaining_quantity(), dec!(700));
    }

    #[test]
    fn test_order_remaining_quantity_fully_filled() {
        let order = Order {
            filled_quantity: dec!(1000),
            ..make_order()
        };
        assert_eq!(order.remaining_quantity(), dec!(0));
    }

    #[test]
    fn test_order_remaining_quantity_overfilled() {
        let order = Order {
            filled_quantity: dec!(1200),
            ..make_order()
        };
        assert_eq!(order.remaining_quantity(), dec!(0));
    }

    #[test]
    fn test_order_can_cancel_active() {
        let order = make_order();
        assert!(order.can_cancel());
    }

    #[test]
    fn test_order_can_cancel_filled() {
        let order = Order {
            status: OrderStatus::Filled,
            ..make_order()
        };
        assert!(!order.can_cancel());
    }

    #[test]
    fn test_order_can_cancel_cancelled() {
        let order = Order {
            status: OrderStatus::Cancelled,
            ..make_order()
        };
        assert!(!order.can_cancel());
    }

    #[test]
    fn test_order_estimated_value_with_price() {
        let order = make_order();
        assert_eq!(order.estimated_value(), Some(dec!(10000.00)));
    }

    #[test]
    fn test_order_estimated_value_no_price() {
        let order = Order {
            price: None,
            ..make_order()
        };
        assert_eq!(order.estimated_value(), None);
    }

    #[test]
    fn test_order_estimated_value_zero_quantity() {
        let order = Order {
            quantity: dec!(0),
            price: Some(dec!(10.00)),
            ..make_order()
        };
        assert_eq!(order.estimated_value(), Some(dec!(0)));
    }

    #[test]
    fn test_order_estimated_value_at_with_market_price() {
        let order = Order {
            quantity: dec!(1000),
            price: None,
            ..make_order()
        };
        assert_eq!(order.estimated_value_at(dec!(10)), dec!(10000));
    }

    // ── Position ──────────────────────────────────────────────────────────────

    #[test]
    fn test_position_pnl_percentage_normal() {
        let pos = Position {
            symbol: "000001.SZ".into(),
            quantity: dec!(1000),
            available_quantity: dec!(1000),
            avg_price: dec!(10.00),
            market_value: dec!(11000),
            unrealized_pnl: dec!(1000),
            realized_pnl: dec!(0),
            updated_at: Utc::now(),
        };
        assert_eq!(pos.pnl_percentage(), dec!(0.1));
    }

    #[test]
    fn test_position_pnl_percentage_negative() {
        let pos = Position {
            quantity: dec!(1000),
            avg_price: dec!(10.00),
            unrealized_pnl: dec!(-500),
            ..make_position()
        };
        assert_eq!(pos.pnl_percentage(), dec!(-0.05));
    }

    #[test]
    fn test_position_pnl_percentage_zero_cost_basis() {
        let pos = Position {
            quantity: dec!(0),
            avg_price: dec!(10.00),
            unrealized_pnl: dec!(100),
            ..make_position()
        };
        assert_eq!(pos.pnl_percentage(), dec!(0));
    }

    #[test]
    fn test_position_frozen_quantity_partial() {
        let pos = Position {
            quantity: dec!(1000),
            available_quantity: dec!(600),
            ..make_position()
        };
        assert_eq!(pos.frozen_quantity(), dec!(400));
    }

    #[test]
    fn test_position_frozen_quantity_none() {
        let pos = Position {
            quantity: dec!(1000),
            available_quantity: dec!(1000),
            ..make_position()
        };
        assert_eq!(pos.frozen_quantity(), dec!(0));
    }

    #[test]
    fn test_position_frozen_quantity_over_available() {
        let pos = Position {
            quantity: dec!(500),
            available_quantity: dec!(1000),
            ..make_position()
        };
        assert_eq!(pos.frozen_quantity(), dec!(0));
    }

    #[test]
    fn test_position_total_pnl_both_positive() {
        let pos = Position {
            realized_pnl: dec!(200),
            unrealized_pnl: dec!(300),
            ..make_position()
        };
        assert_eq!(pos.total_pnl(), dec!(500));
    }

    #[test]
    fn test_position_total_pnl_negative_unrealized() {
        let pos = Position {
            realized_pnl: dec!(200),
            unrealized_pnl: dec!(-100),
            ..make_position()
        };
        assert_eq!(pos.total_pnl(), dec!(100));
    }

    #[test]
    fn test_position_total_pnl_all_zero() {
        let pos = Position {
            realized_pnl: dec!(0),
            unrealized_pnl: dec!(0),
            ..make_position()
        };
        assert_eq!(pos.total_pnl(), dec!(0));
    }

    #[test]
    fn test_position_is_open_with_quantity() {
        let pos = Position {
            quantity: dec!(1000),
            ..make_position()
        };
        assert!(pos.is_open());
    }

    #[test]
    fn test_position_is_open_zero_quantity() {
        let pos = Position {
            quantity: dec!(0),
            ..make_position()
        };
        assert!(!pos.is_open());
    }

    fn make_position() -> Position {
        Position {
            symbol: "000001.SZ".into(),
            quantity: dec!(1000),
            available_quantity: dec!(1000),
            avg_price: dec!(10.00),
            market_value: dec!(11000),
            unrealized_pnl: dec!(1000),
            realized_pnl: dec!(200),
            updated_at: Utc::now(),
        }
    }

    // ── Account ───────────────────────────────────────────────────────────────

    fn make_account() -> Account {
        Account {
            account_id: 0,
            total_assets: dec!(1000000),
            available_cash: dec!(200000),
            frozen_cash: dec!(50000),
            market_value: dec!(750000),
            total_pnl: dec!(50000),
            daily_pnl: dec!(2000),
            margin: dec!(300000),
            margin_ratio: dec!(0.4),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_account_total_equity() {
        let acc = make_account();
        assert_eq!(acc.total_equity(), dec!(1000000));
    }

    #[test]
    fn test_account_total_equity_all_zero() {
        let acc = Account {
            available_cash: dec!(0),
            frozen_cash: dec!(0),
            market_value: dec!(0),
            ..make_account()
        };
        assert_eq!(acc.total_equity(), dec!(0));
    }

    #[test]
    fn test_account_available_margin_normal() {
        let acc = make_account();
        assert_eq!(acc.available_margin(), dec!(700000));
    }

    #[test]
    fn test_account_available_margin_negative() {
        let acc = Account {
            total_assets: dec!(100000),
            margin: dec!(200000),
            ..make_account()
        };
        assert_eq!(acc.available_margin(), dec!(0));
    }

    #[test]
    fn test_account_margin_usage_ratio_normal() {
        let acc = make_account();
        assert_eq!(acc.margin_usage_ratio(), dec!(0.3));
    }

    #[test]
    fn test_account_margin_usage_ratio_zero_assets() {
        let acc = Account {
            total_assets: dec!(0),
            ..make_account()
        };
        assert_eq!(acc.margin_usage_ratio(), dec!(0));
    }

    #[test]
    fn test_account_margin_usage_ratio_over_100_percent() {
        let acc = Account {
            total_assets: dec!(100000),
            margin: dec!(200000),
            ..make_account()
        };
        assert_eq!(acc.margin_usage_ratio(), dec!(1));
    }

    #[test]
    fn test_account_can_cover_sufficient() {
        let acc = make_account();
        assert!(acc.can_cover(dec!(150000)));
    }

    #[test]
    fn test_account_can_cover_exact() {
        let acc = make_account();
        assert!(acc.can_cover(dec!(200000)));
    }

    #[test]
    fn test_account_can_cover_insufficient() {
        let acc = make_account();
        assert!(!acc.can_cover(dec!(250000)));
    }

    #[test]
    fn test_account_can_cover_zero() {
        let acc = make_account();
        assert!(acc.can_cover(dec!(0)));
    }

    // ── StrategyParams ────────────────────────────────────────────────────────

    fn make_strategy_params() -> StrategyParams {
        StrategyParams {
            strategy_id: "strat_1".into(),
            strategy_name: "Test Strategy".into(),
            strategy_type: StrategyType::TrendFollowing,
            params: serde_json::Value::Object(Default::default()),
            enabled: true,
            max_position: dec!(1000),
            max_daily_loss: dec!(50000),
            status: StrategyStatus::Draft,
            description: Some("Test".into()),
            user_id: 0,
            version: 0,
            tags: vec![],
            symbols: vec![],
            instance_label: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_strategyparams_is_valid_normal() {
        let sp = make_strategy_params();
        assert!(sp.is_valid());
    }

    #[test]
    fn test_strategyparams_is_valid_empty_name() {
        let sp = StrategyParams {
            strategy_name: "".into(),
            ..make_strategy_params()
        };
        assert!(!sp.is_valid());
    }

    #[test]
    fn test_strategyparams_is_valid_zero_max_position() {
        let sp = StrategyParams {
            max_position: dec!(0),
            ..make_strategy_params()
        };
        assert!(!sp.is_valid());
    }

    #[test]
    fn test_strategyparams_enable() {
        let mut sp = StrategyParams {
            enabled: false,
            ..make_strategy_params()
        };
        sp.enable();
        assert!(sp.enabled);
    }

    #[test]
    fn test_strategyparams_enable_already_enabled() {
        let mut sp = make_strategy_params();
        sp.enable();
        assert!(sp.enabled);
    }

    #[test]
    fn test_strategyparams_disable() {
        let mut sp = make_strategy_params();
        sp.disable();
        assert!(!sp.enabled);
    }

    #[test]
    fn test_strategyparams_disable_already_disabled() {
        let mut sp = StrategyParams {
            enabled: false,
            ..make_strategy_params()
        };
        sp.disable();
        assert!(!sp.enabled);
    }

    // ── BacktestResult ────────────────────────────────────────────────────────

    fn make_backtest_result() -> BacktestResult {
        BacktestResult {
            id: Some(0),
            strategy_id: "strat_1".into(),
            start_date: Utc::now(),
            end_date: Utc::now() + chrono::Duration::days(30),
            initial_capital: dec!(1000000),
            final_capital: dec!(1200000),
            total_return: dec!(0.2),
            annual_return: dec!(2.4),
            sharpe_ratio: dec!(1.5),
            max_drawdown: dec!(0.15),
            win_rate: dec!(0.55),
            profit_loss_ratio: dec!(1.8),
            total_trades: 100,
            winning_trades: 60,
            losing_trades: 40,
            equity_curve: vec![],
        }
    }

    #[test]
    fn test_backtestresult_profit_factor_normal() {
        let br = make_backtest_result();
        assert_eq!(br.profit_factor(), dec!(1.5));
    }

    #[test]
    fn test_backtestresult_profit_factor_all_wins() {
        let br = BacktestResult {
            total_trades: 10,
            winning_trades: 10,
            losing_trades: 0,
            ..make_backtest_result()
        };
        assert_eq!(br.profit_factor(), Decimal::MAX);
    }

    #[test]
    fn test_backtestresult_profit_factor_no_trades() {
        let br = BacktestResult {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            ..make_backtest_result()
        };
        assert_eq!(br.profit_factor(), dec!(0));
    }

    #[test]
    fn test_backtestresult_net_profit_positive() {
        let br = make_backtest_result();
        assert_eq!(br.net_profit(), dec!(200000));
    }

    #[test]
    fn test_backtestresult_net_profit_negative() {
        let br = BacktestResult {
            initial_capital: dec!(1000000),
            final_capital: dec!(800000),
            ..make_backtest_result()
        };
        assert_eq!(br.net_profit(), dec!(-200000));
    }

    #[test]
    fn test_backtestresult_net_profit_zero() {
        let br = BacktestResult {
            initial_capital: dec!(1000000),
            final_capital: dec!(1000000),
            ..make_backtest_result()
        };
        assert_eq!(br.net_profit(), dec!(0));
    }

    #[test]
    fn test_backtestresult_duration_days() {
        let br = make_backtest_result();
        assert_eq!(br.duration_days(), 30);
    }

    #[test]
    fn test_backtestresult_duration_days_same_day() {
        let now = Utc::now();
        let br = BacktestResult {
            start_date: now,
            end_date: now,
            ..make_backtest_result()
        };
        assert_eq!(br.duration_days(), 0);
    }

    // ── RiskMetrics ───────────────────────────────────────────────────────────

    fn make_risk_metrics() -> RiskMetrics {
        RiskMetrics {
            timestamp: Utc::now(),
            var_95: dec!(-50000),
            var_99: dec!(-80000),
            portfolio_volatility: dec!(0.02),
            beta: dec!(1.0),
            concentration_risk: dec!(0.3),
            leverage: dec!(1.5),
        }
    }

    #[test]
    fn test_riskmetrics_tail_ratio_normal() {
        let rm = make_risk_metrics();
        assert_eq!(rm.tail_ratio(), dec!(1.6));
    }

    #[test]
    fn test_riskmetrics_tail_ratio_positive_var() {
        let rm = RiskMetrics {
            var_95: dec!(10000),
            var_99: dec!(20000),
            ..make_risk_metrics()
        };
        assert_eq!(rm.tail_ratio(), dec!(2));
    }

    #[test]
    fn test_riskmetrics_tail_ratio_zero_var_95() {
        let rm = RiskMetrics {
            var_95: dec!(0),
            ..make_risk_metrics()
        };
        assert_eq!(rm.tail_ratio(), dec!(0));
    }

    #[test]
    fn test_riskmetrics_is_over_leveraged_true() {
        let rm = make_risk_metrics();
        assert!(rm.is_over_leveraged(dec!(1.0)));
    }

    #[test]
    fn test_riskmetrics_is_over_leveraged_false() {
        let rm = make_risk_metrics();
        assert!(!rm.is_over_leveraged(dec!(2.0)));
    }

    #[test]
    fn test_riskmetrics_is_over_leveraged_exact() {
        let rm = make_risk_metrics();
        assert!(!rm.is_over_leveraged(dec!(1.5)));
    }

    // ── Alert ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_alert_new_creates_unacknowledged() {
        let alert = Alert::new(
            AlertLevel::Warning,
            "risk_engine".into(),
            "Margin above 80%".into(),
        );
        assert_eq!(alert.level, AlertLevel::Warning);
        assert_eq!(alert.source, "risk_engine");
        assert_eq!(alert.message, "Margin above 80%");
        assert!(!alert.acknowledged);
    }

    #[test]
    fn test_alert_new_has_zero_id() {
        // New alerts use 0 as placeholder — DB assigns BIGSERIAL on INSERT.
        let alert = Alert::new(AlertLevel::Info, "s".into(), "m".into());
        assert_eq!(alert.alert_id, 0);
    }

    #[test]
    fn test_alert_is_critical_true() {
        let alert = Alert::new(AlertLevel::Critical, "s".into(), "m".into());
        assert!(alert.is_critical());
    }

    #[test]
    fn test_alert_is_critical_warning() {
        let alert = Alert::new(AlertLevel::Warning, "s".into(), "m".into());
        assert!(!alert.is_critical());
    }

    #[test]
    fn test_alert_is_critical_info() {
        let alert = Alert::new(AlertLevel::Info, "s".into(), "m".into());
        assert!(!alert.is_critical());
    }

    #[test]
    fn test_alert_age_seconds_nonnegative() {
        let alert = Alert::new(AlertLevel::Info, "s".into(), "m".into());
        assert!(alert.age_seconds() >= 0);
    }

    #[test]
    fn test_alert_acknowledge() {
        let mut alert = Alert::new(AlertLevel::Warning, "s".into(), "m".into());
        assert!(!alert.acknowledged);
        alert.acknowledge();
        assert!(alert.acknowledged);
    }

    // ── StrategyStatus ────────────────────────────────────────────────────────

    #[test]
    fn test_strategy_status_from_str_all_variants() {
        assert_eq!(
            "Draft".parse::<StrategyStatus>().unwrap(),
            StrategyStatus::Draft
        );
        assert_eq!(
            "Backtesting".parse::<StrategyStatus>().unwrap(),
            StrategyStatus::Backtesting
        );
        assert_eq!(
            "Deployed".parse::<StrategyStatus>().unwrap(),
            StrategyStatus::Deployed
        );
        assert_eq!(
            "Running".parse::<StrategyStatus>().unwrap(),
            StrategyStatus::Running
        );
        assert_eq!(
            "Paused".parse::<StrategyStatus>().unwrap(),
            StrategyStatus::Paused
        );
        assert_eq!(
            "Archived".parse::<StrategyStatus>().unwrap(),
            StrategyStatus::Archived
        );
    }

    #[test]
    fn test_strategy_status_from_str_unknown_defaults_to_draft() {
        assert_eq!(
            "unknown".parse::<StrategyStatus>().unwrap(),
            StrategyStatus::Draft
        );
        assert_eq!("".parse::<StrategyStatus>().unwrap(), StrategyStatus::Draft);
    }

    #[test]
    fn test_strategy_status_can_transition_draft_to_backtesting() {
        assert!(StrategyStatus::Draft.can_transition_to(StrategyStatus::Backtesting));
    }

    #[test]
    fn test_strategy_status_can_transition_draft_to_archived() {
        assert!(StrategyStatus::Draft.can_transition_to(StrategyStatus::Archived));
    }

    #[test]
    fn test_strategy_status_can_transition_draft_to_deployed() {
        assert!(StrategyStatus::Draft.can_transition_to(StrategyStatus::Deployed));
    }

    #[test]
    fn test_strategy_status_can_transition_draft_to_running_blocked() {
        assert!(!StrategyStatus::Draft.can_transition_to(StrategyStatus::Running));
    }

    #[test]
    fn test_strategy_status_can_transition_backtesting_to_deployed() {
        assert!(StrategyStatus::Backtesting.can_transition_to(StrategyStatus::Deployed));
    }

    #[test]
    fn test_strategy_status_can_transition_backtesting_to_draft() {
        assert!(StrategyStatus::Backtesting.can_transition_to(StrategyStatus::Draft));
    }

    #[test]
    fn test_strategy_status_can_transition_deployed_to_running() {
        assert!(StrategyStatus::Deployed.can_transition_to(StrategyStatus::Running));
    }

    #[test]
    fn test_strategy_status_can_transition_running_to_paused() {
        assert!(StrategyStatus::Running.can_transition_to(StrategyStatus::Paused));
    }

    #[test]
    fn test_strategy_status_can_transition_running_to_archived() {
        assert!(StrategyStatus::Running.can_transition_to(StrategyStatus::Archived));
    }

    #[test]
    fn test_strategy_status_can_transition_paused_to_running() {
        assert!(StrategyStatus::Paused.can_transition_to(StrategyStatus::Running));
    }

    #[test]
    fn test_strategy_status_can_transition_paused_to_archived() {
        assert!(StrategyStatus::Paused.can_transition_to(StrategyStatus::Archived));
    }

    #[test]
    fn test_strategy_status_transition_to_valid() {
        let mut sp = make_strategy_params();
        assert_eq!(sp.status, StrategyStatus::Draft);
        let new_status = sp.transition_to(StrategyStatus::Backtesting).unwrap();
        assert_eq!(new_status, StrategyStatus::Backtesting);
        assert_eq!(sp.status, StrategyStatus::Backtesting);
    }

    #[test]
    fn test_strategy_status_transition_to_invalid() {
        let mut sp = make_strategy_params();
        sp.status = StrategyStatus::Draft;
        let result = sp.transition_to(StrategyStatus::Running);
        assert!(result.is_err());
        assert_eq!(sp.status, StrategyStatus::Draft);
    }

    // ── StrategyParams new fields ─────────────────────────────────────────────

    #[test]
    fn test_strategy_params_with_tags_and_symbols() {
        let sp = StrategyParams {
            tags: vec!["momentum".into(), "trend".into()],
            symbols: vec!["BTC-USDT".into(), "ETH-USDT".into()],
            ..make_strategy_params()
        };
        assert_eq!(sp.tags.len(), 2);
        assert_eq!(sp.symbols.len(), 2);
    }

    #[test]
    fn test_strategy_params_serialization_roundtrip() {
        let sp = StrategyParams {
            status: StrategyStatus::Running,
            description: Some("Test strategy".into()),
            tags: vec!["tag1".into()],
            symbols: vec!["SYM-USDT".into()],
            ..make_strategy_params()
        };
        let json = serde_json::to_string(&sp).unwrap();
        let deserialized: StrategyParams = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, StrategyStatus::Running);
        assert_eq!(deserialized.description, Some("Test strategy".into()));
        assert_eq!(deserialized.tags, vec![String::from("tag1")]);
        assert_eq!(deserialized.symbols, vec![String::from("SYM-USDT")]);
    }

    #[test]
    fn test_strategy_params_default_status_is_draft() {
        let json = r#"{"strategy_id":"x","strategy_name":"X","strategy_type":"Custom","params":{},"enabled":false,"max_position":0,"max_daily_loss":0,"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#;
        let sp: StrategyParams = serde_json::from_str(json).unwrap();
        assert_eq!(sp.status, StrategyStatus::Draft);
        assert!(sp.tags.is_empty());
        assert!(sp.symbols.is_empty());
    }
}
