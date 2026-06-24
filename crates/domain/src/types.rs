use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub order_id: Uuid,
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

    /// Create a new pending order with generated UUID.
    pub fn new(
        strategy_id: String,
        symbol: String,
        order_type: OrderType,
        side: OrderSide,
        price: Option<Decimal>,
        quantity: Decimal,
    ) -> Self {
        Self {
            order_id: Uuid::new_v4(),
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
    pub account_id: Uuid,
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
}

// ─── Backtest Result ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
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
    pub alert_id: Uuid,
    pub level: AlertLevel,
    pub source: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

impl Alert {
    /// Create a new unacknowledged alert.
    pub fn new(level: AlertLevel, source: String, message: String) -> Self {
        Self {
            alert_id: Uuid::new_v4(),
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
            order_id: Uuid::new_v4(),
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
    fn test_order_new_generates_unique_id() {
        let o1 = Order::new(
            "s".into(),
            "sym".into(),
            OrderType::Market,
            OrderSide::Sell,
            None,
            dec!(100),
        );
        let o2 = Order::new(
            "s".into(),
            "sym".into(),
            OrderType::Market,
            OrderSide::Sell,
            None,
            dec!(100),
        );
        assert_ne!(o1.order_id, o2.order_id);
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
            account_id: Uuid::new_v4(),
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
    fn test_alert_new_generates_unique_id() {
        let a1 = Alert::new(AlertLevel::Info, "s".into(), "m".into());
        let a2 = Alert::new(AlertLevel::Info, "s".into(), "m".into());
        assert_ne!(a1.alert_id, a2.alert_id);
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
}
