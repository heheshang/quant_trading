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
