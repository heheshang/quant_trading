//! Test / demo data for Binance (feature = `test-utils`).

use crate::types::{BinanceBalance, BinanceKline, BinanceOrder, BinanceOrderBook, BinancePosition};
use rust_decimal::Decimal;

/// Build a representative kline row (the raw array response shape).
pub fn kline_row(open: &str, close: &str) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!(1_700_000_000_000i64),
        serde_json::json!(open),
        serde_json::json!("105.0"),
        serde_json::json!("95.0"),
        serde_json::json!(close),
        serde_json::json!("1200"),
        serde_json::json!(1_700_000_005_999i64),
        serde_json::json!("123456"),
        serde_json::json!(42),
    ]
}

/// A parsed kline for assertions.
pub fn sample_kline() -> BinanceKline {
    BinanceKline {
        open_time: 1_700_000_000_000,
        open: Decimal::new(10000, 2),
        high: Decimal::new(10500, 2),
        low: Decimal::new(9500, 2),
        close: Decimal::new(10300, 2),
        volume: Decimal::new(1200, 0),
        close_time: 1_700_000_005_999,
        quote_volume: Decimal::new(123456, 0),
        trades: 42,
    }
}

pub fn sample_balance() -> Vec<BinanceBalance> {
    vec![
        BinanceBalance {
            asset: "BTC".to_string(),
            free: Decimal::new(10000, 4),
            locked: Decimal::new(0, 0),
        },
        BinanceBalance {
            asset: "USDT".to_string(),
            free: Decimal::new(500000, 2),
            locked: Decimal::new(0, 0),
        },
    ]
}

pub fn sample_order_book() -> BinanceOrderBook {
    #[allow(clippy::inconsistent_digit_grouping)]
    BinanceOrderBook {
        symbol: "BTCUSDT".to_string(),
        bids: vec![(Decimal::new(10000, 0), Decimal::new(1, 0))],
        asks: vec![(Decimal::new(10001, 0), Decimal::new(2, 0))],
    }
}

pub fn sample_order() -> BinanceOrder {
    BinanceOrder {
        symbol: "BTCUSDT".to_string(),
        order_id: 123,
        client_order_id: "ord-x".to_string(),
        status: "NEW".to_string(),
        executed_qty: Decimal::new(5, 3),
        cummulative_quote_qty: Decimal::new(250, 0),
        price: Decimal::new(50_000, 0),
        side: "BUY".to_string(),
        order_type: "LIMIT".to_string(),
        orig_qty: Decimal::new(1, 2),
        time: 1_700_000_000_000,
        update_time: 1_700_000_001_000,
    }
}

pub fn sample_position() -> BinancePosition {
    BinancePosition {
        symbol: "BTCUSDT".to_string(),
        position_amt: Decimal::new(10, 4),
        entry_price: Decimal::new(50_000, 0),
        mark_price: Decimal::new(51_000, 0),
        un_realized_profit: Decimal::new(1, 0),
        liquidation_price: Decimal::ZERO,
        leverage: "10".to_string(),
        margin_type: "crossed".to_string(),
        notional: Decimal::new(50, 0),
        position_side: "BOTH".to_string(),
    }
}
