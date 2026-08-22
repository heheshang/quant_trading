//! Mock data for order types (Order / PlaceOrderRequest).

use crate::types::*;

/// Create a default [`OkxOrder`] for testing.
pub fn mock_okx_order(inst_id: &str, side: &str) -> OkxOrder {
    OkxOrder {
        ord_id: "123456789".to_string(),
        cl_ord_id: "cl-123456789".to_string(),
        inst_id: inst_id.to_string(),
        side: side.to_string(),
        ord_type: "limit".to_string(),
        px: "45000.0".to_string(),
        sz: "1".to_string(),
        state: "live".to_string(),
        avg_px: "0".to_string(),
        acc_fill_sz: "0".to_string(),
        fee: "0".to_string(),
        u_time: super::default_ts().to_string(),
    }
}

/// Create a fully filled order.
pub fn mock_filled_order(inst_id: &str, side: &str) -> OkxOrder {
    OkxOrder {
        ord_id: "987654321".to_string(),
        cl_ord_id: "cl-filled".to_string(),
        inst_id: inst_id.to_string(),
        side: side.to_string(),
        ord_type: "market".to_string(),
        px: "0".to_string(),
        sz: "1".to_string(),
        state: "filled".to_string(),
        avg_px: "45000.5".to_string(),
        acc_fill_sz: "1".to_string(),
        fee: "0".to_string(),
        u_time: super::default_ts().to_string(),
    }
}

/// Create a canceled order.
pub fn mock_canceled_order(inst_id: &str, side: &str) -> OkxOrder {
    OkxOrder {
        ord_id: "555555555".to_string(),
        cl_ord_id: "cl-canceled".to_string(),
        inst_id: inst_id.to_string(),
        side: side.to_string(),
        ord_type: "limit".to_string(),
        px: "44000.0".to_string(),
        sz: "2".to_string(),
        state: "canceled".to_string(),
        avg_px: "0".to_string(),
        acc_fill_sz: "0".to_string(),
        fee: "0".to_string(),
        u_time: super::default_ts().to_string(),
    }
}

/// Create a default market buy [`OkxPlaceOrderRequest`] for testing.
pub fn mock_place_order_request(inst_id: &str, side: &str, sz: &str) -> OkxPlaceOrderRequest {
    OkxPlaceOrderRequest {
        inst_id: inst_id.to_string(),
        td_mode: "cross".to_string(),
        side: side.to_string(),
        ord_type: "market".to_string(),
        sz: sz.to_string(),
        px: None,
        cl_ord_id: None,
        tag: None,
        pos_side: None,
        ccy: None,
        px_usd: None,
        px_vol: None,
        reduce_only: None,
        tgt_ccy: None,
    }
}

/// Create a default limit order [`OkxPlaceOrderRequest`] for testing.
pub fn mock_limit_order_request(
    inst_id: &str,
    side: &str,
    sz: &str,
    px: &str,
) -> OkxPlaceOrderRequest {
    OkxPlaceOrderRequest {
        inst_id: inst_id.to_string(),
        td_mode: "cross".to_string(),
        side: side.to_string(),
        ord_type: "limit".to_string(),
        sz: sz.to_string(),
        px: Some(px.to_string()),
        cl_ord_id: Some("cl-test-123".to_string()),
        tag: Some("test".to_string()),
        pos_side: None,
        ccy: None,
        px_usd: None,
        px_vol: None,
        reduce_only: None,
        tgt_ccy: None,
    }
}
