//! Mock data for market-data types (Candle / Ticker / FundingRate / MarkPrice /
//! IndexPrice / OpenInterest / Trade).

use crate::types::*;

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
    mock_okx_candle(
        super::default_ts(),
        "45000",
        "45500",
        "44900",
        "45200",
        "100.0",
    )
}

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
        ts: super::default_ts().to_string(),
    }
}

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

/// Create a default [`OkxMarkPrice`] for testing.
pub fn mock_okx_mark_price(inst_id: &str) -> OkxMarkPrice {
    OkxMarkPrice {
        inst_id: inst_id.to_string(),
        mark_px: "45200.0".to_string(),
        ts: super::default_ts().to_string(),
    }
}

/// Create a default [`OkxIndexPrice`] for testing.
pub fn mock_okx_index_price(inst_id: &str) -> OkxIndexPrice {
    OkxIndexPrice {
        inst_id: inst_id.to_string(),
        idx_px: "45205.0".to_string(),
        ts: super::default_ts().to_string(),
    }
}

/// Create a default [`OkxOpenInterest`] for testing.
pub fn mock_okx_open_interest(inst_id: &str) -> OkxOpenInterest {
    OkxOpenInterest {
        inst_id: inst_id.to_string(),
        oi: "50000".to_string(),
        oi_ccy: "45000".to_string(),
        ts: super::default_ts().to_string(),
    }
}

/// Create a default [`OkxTrade`] for testing.
pub fn mock_okx_trade(inst_id: &str) -> OkxTrade {
    OkxTrade {
        inst_id: inst_id.to_string(),
        trade_id: "123456".to_string(),
        px: "45200.0".to_string(),
        sz: "0.5".to_string(),
        side: "buy".to_string(),
        ts: super::default_ts().to_string(),
    }
}
