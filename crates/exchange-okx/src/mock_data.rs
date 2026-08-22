//! Mock data factory functions for OKX types.
//!
//! These are pure data constructors used in tests. They provide
//! default instances with sensible test values. No I/O, no mocking
//! framework, no random or non-deterministic values.

use crate::types::*;
use okx::api::announcements::announcements_api::{AnnouncementDetail, AnnouncementPage};

// ── Helpers ──

fn default_ts() -> &'static str {
    "1597026383085"
}

// ── Balance ──

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

// ── Position ──

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

// ── Order ──

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
        u_time: default_ts().to_string(),
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
        u_time: default_ts().to_string(),
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
        u_time: default_ts().to_string(),
    }
}

// ── Candle ──

/// Create a single [`OkxCandle`] for testing.
pub fn mock_okx_candle(
    ts: &str,
    open: &str,
    high: &str,
    low: &str,
    close: &str,
    vol: &str,
) -> OkxCandle {
    let vol_ccy = {
        let v: f64 = vol.parse().unwrap_or(0.0);
        format!("{:.0}", v * 45000.0)
    };
    OkxCandle {
        ts: ts.to_string(),
        open: open.to_string(),
        high: high.to_string(),
        low: low.to_string(),
        close: close.to_string(),
        vol: vol.to_string(),
        vol_ccy,
    }
}

/// Create `count` candles with sequential timestamps.
///
/// Each candle is 1 hour apart, simulating a rising-then-falling price
/// series. The first candle starts at `ts_start`.
pub fn mock_okx_candles(count: usize) -> Vec<OkxCandle> {
    let base: u64 = 1597026383000;
    let mut candles = Vec::with_capacity(count);
    for i in 0..count {
        let ts = (base + i as u64 * 3600000).to_string();
        let mid = count / 2;
        let price_offset: f64 = if i < mid {
            (i as f64) * 100.0 // rising
        } else {
            ((count - i) as f64) * 100.0 // falling
        };
        let open = 45000.0 + price_offset;
        let high = open + 50.0;
        let low = open - 50.0;
        let close = open + 10.0;
        candles.push(mock_okx_candle(
            &ts,
            &open.to_string(),
            &high.to_string(),
            &low.to_string(),
            &close.to_string(),
            "100.0",
        ));
    }
    candles
}

/// Create a single candle with default values.
pub fn mock_default_candle() -> OkxCandle {
    mock_okx_candle(default_ts(), "45000", "45500", "44900", "45200", "100.0")
}

// ── Ticker ──

/// Create a default [`OkxTicker`] for testing.
pub fn mock_okx_ticker(inst_id: &str) -> OkxTicker {
    OkxTicker {
        inst_id: inst_id.to_string(),
        last: "45200.0".to_string(),
        last_sz: "1.5".to_string(),
        ask_px: "45210.0".to_string(),
        bid_px: "45190.0".to_string(),
        open_24h: "44800.0".to_string(),
        high_24h: "46000.0".to_string(),
        low_24h: "44500.0".to_string(),
        vol_ccy_24h: "150000000".to_string(),
        vol_24h: "3333.3".to_string(),
        sod_utc0: "44900.0".to_string(),
        sod_utc8: "45000.0".to_string(),
        ts: default_ts().to_string(),
    }
}

// ── Funding Rate ──

/// Create a default [`OkxFundingRate`] for testing.
pub fn mock_okx_funding_rate(inst_id: &str) -> OkxFundingRate {
    OkxFundingRate {
        inst_id: inst_id.to_string(),
        funding_rate: "0.0001".to_string(),
        next_funding_rate: "0.00015".to_string(),
        funding_time: "1597026383085".to_string(),
        inst_type: "SWAP".to_string(),
    }
}

// ── Mark Price ──

/// Create a default [`OkxMarkPrice`] for testing.
pub fn mock_okx_mark_price(inst_id: &str) -> OkxMarkPrice {
    OkxMarkPrice {
        inst_id: inst_id.to_string(),
        mark_px: "45200.0".to_string(),
        ts: default_ts().to_string(),
    }
}

// ── Index Price ──

/// Create a default [`OkxIndexPrice`] for testing.
pub fn mock_okx_index_price(inst_id: &str) -> OkxIndexPrice {
    OkxIndexPrice {
        inst_id: inst_id.to_string(),
        idx_px: "45205.0".to_string(),
        ts: default_ts().to_string(),
    }
}

// ── Open Interest ──

/// Create a default [`OkxOpenInterest`] for testing.
pub fn mock_okx_open_interest(inst_id: &str) -> OkxOpenInterest {
    OkxOpenInterest {
        inst_id: inst_id.to_string(),
        oi: "50000".to_string(),
        oi_ccy: "45000".to_string(),
        ts: default_ts().to_string(),
    }
}

// ── Trade ──

/// Create a default [`OkxTrade`] for testing.
pub fn mock_okx_trade(inst_id: &str) -> OkxTrade {
    OkxTrade {
        inst_id: inst_id.to_string(),
        trade_id: "123456".to_string(),
        px: "45200.0".to_string(),
        sz: "0.5".to_string(),
        side: "buy".to_string(),
        ts: default_ts().to_string(),
    }
}

// ── Order Book ──

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
        ts: default_ts().to_string(),
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
        ts: default_ts().to_string(),
    }
}

/// Create an [`OkxOrderBook`] with empty order books (no asks or bids).
pub fn mock_empty_order_book() -> OkxOrderBook {
    OkxOrderBook {
        asks: vec![],
        bids: vec![],
        ts: default_ts().to_string(),
    }
}

// ── Announcements ──

/// Create a default [`AnnouncementDetail`] for testing.
pub fn mock_announcement_detail() -> AnnouncementDetail {
    AnnouncementDetail {
        ann_type: "delisting".to_string(),
        p_time: "1597026383085".to_string(),
        title: "Test Announcement".to_string(),
        url: "https://www.okx.com/support/announcement/test".to_string(),
    }
}

/// Create a default [`AnnouncementPage`] for testing.
pub fn mock_announcement_page() -> AnnouncementPage {
    AnnouncementPage {
        details: vec![
            mock_announcement_detail(),
            AnnouncementDetail {
                ann_type: "listing".to_string(),
                p_time: "1597026383086".to_string(),
                title: "New Token Listing".to_string(),
                url: "https://www.okx.com/support/announcement/listing".to_string(),
            },
        ],
        total_page: "1".to_string(),
    }
}

// ── Place Order Request ──

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_okx_balance() {
        let b = mock_okx_balance("BTC", "1.5");
        assert_eq!(b.ccy, "BTC");
        assert_eq!(b.eq, "1.5");
        assert_eq!(b.cash_bal, "1.5");
        assert_eq!(b.avail_eq, "1.5");
        assert_eq!(b.frozen_bal, "0");
    }

    #[test]
    fn test_mock_large_number_balance() {
        let b = mock_large_number_balance();
        assert_eq!(b.eq, "123456789012345678");
    }

    #[test]
    fn test_mock_okx_balance_list() {
        let list = mock_okx_balance_list();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].ccy, "BTC");
        assert_eq!(list[1].ccy, "ETH");
        assert_eq!(list[2].ccy, "USDT");
    }

    #[test]
    fn test_mock_empty_balance() {
        let b = mock_empty_balance();
        assert_eq!(b.eq, "0");
    }

    #[test]
    fn test_mock_okx_position() {
        let p = mock_okx_position("BTC-USDT", "1");
        assert_eq!(p.inst_id, "BTC-USDT");
        assert_eq!(p.pos, "1");
    }

    #[test]
    fn test_mock_okx_position_list() {
        let list = mock_okx_position_list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].inst_id, "BTC-USDT");
        assert_eq!(list[1].inst_id, "ETH-USDT");
    }

    #[test]
    fn test_mock_zero_position() {
        let p = mock_zero_position("BTC-USDT");
        assert_eq!(p.pos, "0");
    }

    #[test]
    fn test_mock_okx_order() {
        let o = mock_okx_order("BTC-USDT", "buy");
        assert_eq!(o.inst_id, "BTC-USDT");
        assert_eq!(o.side, "buy");
        assert_eq!(o.state, "live");
    }

    #[test]
    fn test_mock_filled_order() {
        let o = mock_filled_order("BTC-USDT", "sell");
        assert_eq!(o.state, "filled");
        assert_eq!(o.acc_fill_sz, "1");
    }

    #[test]
    fn test_mock_canceled_order() {
        let o = mock_canceled_order("BTC-USDT", "buy");
        assert_eq!(o.state, "canceled");
    }

    #[test]
    fn test_mock_okx_candle() {
        let c = mock_okx_candle("1597026383085", "45000", "45500", "44900", "45200", "100.0");
        assert_eq!(c.ts, "1597026383085");
        assert_eq!(c.open, "45000");
        assert_eq!(c.close, "45200");
    }

    #[test]
    fn test_mock_okx_candles() {
        let candles = mock_okx_candles(5);
        assert_eq!(candles.len(), 5);
        // Verify sequential timestamps
        let ts0: u64 = candles[0].ts.parse().unwrap();
        let ts1: u64 = candles[1].ts.parse().unwrap();
        assert_eq!(ts1 - ts0, 3600000);
    }

    #[test]
    fn test_mock_default_candle() {
        let c = mock_default_candle();
        assert_eq!(c.ts, default_ts());
    }

    #[test]
    fn test_mock_okx_ticker() {
        let t = mock_okx_ticker("BTC-USDT");
        assert_eq!(t.inst_id, "BTC-USDT");
        assert_eq!(t.last, "45200.0");
    }

    #[test]
    fn test_mock_okx_funding_rate() {
        let f = mock_okx_funding_rate("BTC-USDT-SWAP");
        assert_eq!(f.inst_id, "BTC-USDT-SWAP");
        assert_eq!(f.funding_rate, "0.0001");
    }

    #[test]
    fn test_mock_okx_mark_price() {
        let m = mock_okx_mark_price("BTC-USDT");
        assert_eq!(m.inst_id, "BTC-USDT");
        assert_eq!(m.mark_px, "45200.0");
    }

    #[test]
    fn test_mock_okx_index_price() {
        let p = mock_okx_index_price("BTC-USDT");
        assert_eq!(p.inst_id, "BTC-USDT");
        assert_eq!(p.idx_px, "45205.0");
    }

    #[test]
    fn test_mock_okx_open_interest() {
        let oi = mock_okx_open_interest("BTC-USDT");
        assert_eq!(oi.inst_id, "BTC-USDT");
        assert_eq!(oi.oi, "50000");
    }

    #[test]
    fn test_mock_okx_trade() {
        let t = mock_okx_trade("BTC-USDT");
        assert_eq!(t.inst_id, "BTC-USDT");
        assert_eq!(t.px, "45200.0");
        assert_eq!(t.side, "buy");
    }

    #[test]
    fn test_mock_okx_order_book() {
        let ob = mock_okx_order_book();
        assert_eq!(ob.asks.len(), 3);
        assert_eq!(ob.bids.len(), 3);
        assert_eq!(ob.asks[0][0], "45210.0");
        assert_eq!(ob.bids[0][0], "45190.0");
    }

    #[test]
    fn test_mock_single_level_order_book() {
        let ob = mock_single_level_order_book();
        assert_eq!(ob.asks.len(), 1);
        assert_eq!(ob.bids.len(), 1);
    }

    #[test]
    fn test_mock_empty_order_book() {
        let ob = mock_empty_order_book();
        assert!(ob.asks.is_empty());
        assert!(ob.bids.is_empty());
    }

    #[test]
    fn test_mock_announcement_detail() {
        let d = mock_announcement_detail();
        assert_eq!(d.ann_type, "delisting");
        assert_eq!(d.title, "Test Announcement");
    }

    #[test]
    fn test_mock_announcement_page() {
        let p = mock_announcement_page();
        assert_eq!(p.details.len(), 2);
        assert_eq!(p.total_page, "1");
    }

    #[test]
    fn test_mock_place_order_request() {
        let r = mock_place_order_request("BTC-USDT", "buy", "1");
        assert_eq!(r.inst_id, "BTC-USDT");
        assert_eq!(r.side, "buy");
        assert_eq!(r.ord_type, "market");
        assert!(r.px.is_none());
    }

    #[test]
    fn test_mock_limit_order_request() {
        let r = mock_limit_order_request("BTC-USDT", "sell", "0.5", "46000");
        assert_eq!(r.inst_id, "BTC-USDT");
        assert_eq!(r.side, "sell");
        assert_eq!(r.ord_type, "limit");
        assert_eq!(r.px, Some("46000".to_string()));
        assert_eq!(r.cl_ord_id, Some("cl-test-123".to_string()));
    }
}
