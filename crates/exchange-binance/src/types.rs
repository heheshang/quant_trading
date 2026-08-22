//! Binance-specific data types and symbol helpers.
//!
//! Mirrors `exchange-okx::types`: defines the DTOs parsed from Binance REST
//! responses plus a small symbol conversion utility (Binance uses `BTCUSDT`
//! while the app's domain uses `BTC-USDT`).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Trading environment selects the REST base URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinanceEnvironment {
    Spot,
    Futures,
}

impl BinanceEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Futures => "futures",
        }
    }

    /// REST base URL for this environment.
    pub fn base_url(&self) -> &'static str {
        match self {
            Self::Spot => "https://api.binance.com",
            Self::Futures => "https://fapi.binance.com",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value == "futures" {
            Self::Futures
        } else {
            Self::Spot
        }
    }
}

/// Binance kline/candle row (`/api/v3/klines`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceKline {
    pub open_time: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub close_time: i64,
    pub quote_volume: Decimal,
    pub trades: u64,
}

/// Binance account balance entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceBalance {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
}

/// Binance order-book depth (`/api/v3/depth`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceOrderBook {
    pub symbol: String,
    pub bids: Vec<(Decimal, Decimal)>,
    pub asks: Vec<(Decimal, Decimal)>,
}

/// Request to place a Binance order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinancePlaceOrderRequest {
    pub symbol: String,
    pub side: BinanceSide,
    pub order_type: BinanceOrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
}

/// Trade side.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BinanceSide {
    Buy,
    Sell,
}

/// Order type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BinanceOrderType {
    Market,
    Limit,
}

/// Normalized Binance order result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinanceOrder {
    pub symbol: String,
    pub order_id: i64,
    pub client_order_id: String,
    pub status: String,
    pub executed_qty: Decimal,
    pub cummulative_quote_qty: Decimal,
    pub price: Decimal,
}

/// Convert app-style `BTC-USDT` to Binance `BTCUSDT`.
pub fn to_binance_symbol(domain: &str) -> String {
    domain.replace('-', "")
}

/// Best-effort conversion of Binance `BTCUSDT` back to domain `BTC-USDT`.
///
/// Inserts `-` before a known quote asset to match the app's symbol format.
pub fn from_binance_symbol(binance: &str) -> String {
    const QUOTES: [&str; 6] = ["USDT", "USDC", "BUSD", "BTC", "ETH", "FDUSD"];
    for q in QUOTES {
        if let Some(base) = binance.strip_suffix(q) {
            if !base.is_empty() {
                return format!("{}-{}", base, q);
            }
        }
    }
    binance.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_domain_to_binance_symbol() {
        assert_eq!(to_binance_symbol("BTC-USDT"), "BTCUSDT");
        assert_eq!(to_binance_symbol("ETH-USDC"), "ETHUSDC");
    }

    #[test]
    fn converts_binance_to_domain_symbol() {
        assert_eq!(from_binance_symbol("BTCUSDT"), "BTC-USDT");
        assert_eq!(from_binance_symbol("ETHUSDT"), "ETH-USDT");
        assert_eq!(from_binance_symbol("BTCBTC"), "BTC-BTC");
    }

    #[test]
    fn environment_base_url() {
        assert_eq!(BinanceEnvironment::Spot.base_url(), "https://api.binance.com");
        assert_eq!(
            BinanceEnvironment::Futures.base_url(),
            "https://fapi.binance.com"
        );
        assert_eq!(BinanceEnvironment::parse("futures"), BinanceEnvironment::Futures);
        assert_eq!(BinanceEnvironment::parse("spot"), BinanceEnvironment::Spot);
    }
}
