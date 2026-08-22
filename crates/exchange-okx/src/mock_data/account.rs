//! Mock data for account-related types (Balance / Position).

use crate::types::*;

/// Create a default [`OkxBalance`] for testing.
pub fn mock_okx_balance(ccy: &str, eq: &str) -> OkxBalance {
    OkxBalance {
        ccy: ccy.to_string(),
        eq: eq.to_string(),
        cash_bal: eq.to_string(),
        avail_eq: eq.to_string(),
        frozen_bal: "0".to_string(),
    }
}

/// Create an [`OkxBalance`] with a value exceeding 2^53 for precision testing.
pub fn mock_large_number_balance() -> OkxBalance {
    OkxBalance {
        ccy: "BTC".to_string(),
        eq: "123456789012345678".to_string(),
        cash_bal: "123456789012345678".to_string(),
        avail_eq: "123456789012345678".to_string(),
        frozen_bal: "0".to_string(),
    }
}

/// Create a list of three default balances (BTC, ETH, USDT).
pub fn mock_okx_balance_list() -> Vec<OkxBalance> {
    vec![
        mock_okx_balance("BTC", "1.5"),
        mock_okx_balance("ETH", "10.0"),
        mock_okx_balance("USDT", "50000.0"),
    ]
}

/// Create an empty-balance variant (all zero).
pub fn mock_empty_balance() -> OkxBalance {
    mock_okx_balance("USDT", "0")
}

/// Create a default [`OkxPosition`] for testing.
pub fn mock_okx_position(inst_id: &str, pos: &str) -> OkxPosition {
    OkxPosition {
        inst_id: inst_id.to_string(),
        pos: pos.to_string(),
        avail_pos: pos.to_string(),
        avg_px: "45000.0".to_string(),
        upl: "100.0".to_string(),
        upl_ratio: "0.02".to_string(),
    }
}

/// Create a list of two positions (long BTC, short ETH).
pub fn mock_okx_position_list() -> Vec<OkxPosition> {
    vec![
        mock_okx_position("BTC-USDT", "1"),
        OkxPosition {
            inst_id: "ETH-USDT".to_string(),
            pos: "-5".to_string(),
            avail_pos: "-5".to_string(),
            avg_px: "3200.0".to_string(),
            upl: "-50.0".to_string(),
            upl_ratio: "-0.01".to_string(),
        },
    ]
}

/// Create a zero-position variant.
pub fn mock_zero_position(inst_id: &str) -> OkxPosition {
    mock_okx_position(inst_id, "0")
}
