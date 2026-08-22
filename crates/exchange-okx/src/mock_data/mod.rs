//! Mock data factory functions for OKX types.
//!
//! These are pure data constructors used in tests. They provide
//! default instances with sensible test values. No I/O, no mocking
//! framework, no random or non-deterministic values.
//!
//! 按数据类型拆分为子模块，`mod.rs` 仅承担 re-export + 共享 helper + 测试。

pub mod account;
pub mod announcements;
pub mod market;
pub mod order;
pub mod order_book;

pub use account::*;
pub use announcements::*;
pub use market::*;
pub use order::*;
pub use order_book::*;

fn default_ts() -> &'static str {
    "1597026383085"
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
