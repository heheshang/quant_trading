//! Mock data for order-book types.

use crate::types::*;

/// Create a default [`OkxOrderBook`] for testing.
///
/// Contains 3 price levels on each side at spreads of 10, 20, 30.
pub fn mock_okx_order_book() -> OkxOrderBook {
    OkxOrderBook {
        asks: vec![
            vec![
                "45210.0".to_string(),
                "1.0".to_string(),
                "0".to_string(),
                "1".to_string(),
            ],
            vec![
                "45220.0".to_string(),
                "2.0".to_string(),
                "0".to_string(),
                "1".to_string(),
            ],
            vec![
                "45230.0".to_string(),
                "3.0".to_string(),
                "0".to_string(),
                "1".to_string(),
            ],
        ],
        bids: vec![
            vec![
                "45190.0".to_string(),
                "1.5".to_string(),
                "0".to_string(),
                "1".to_string(),
            ],
            vec![
                "45180.0".to_string(),
                "2.5".to_string(),
                "0".to_string(),
                "1".to_string(),
            ],
            vec![
                "45170.0".to_string(),
                "3.5".to_string(),
                "0".to_string(),
                "1".to_string(),
            ],
        ],
        ts: super::default_ts().to_string(),
    }
}

/// Create an [`OkxOrderBook`] with a single price level on each side.
pub fn mock_single_level_order_book() -> OkxOrderBook {
    OkxOrderBook {
        asks: vec![vec![
            "45210.0".to_string(),
            "1.0".to_string(),
            "0".to_string(),
            "1".to_string(),
        ]],
        bids: vec![vec![
            "45190.0".to_string(),
            "1.5".to_string(),
            "0".to_string(),
            "1".to_string(),
        ]],
        ts: super::default_ts().to_string(),
    }
}

/// Create an [`OkxOrderBook`] with empty order books (no asks or bids).
pub fn mock_empty_order_book() -> OkxOrderBook {
    OkxOrderBook {
        asks: vec![],
        bids: vec![],
        ts: super::default_ts().to_string(),
    }
}
