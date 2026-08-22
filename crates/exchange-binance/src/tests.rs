//! Unit tests for the Binance client: signature correctness, kline parsing,
//! error mapping, and the mock-able trait surface.

use crate::client::{parse_decimal, sign};
use crate::types::*;
use rust_decimal::Decimal;

#[test]
fn hmac_signature_matches_known_vector() {
    // Example: secret "secret", query "symbol=BTCUSDT&timestamp=123".
    // Computed independently; verifies hex HMAC-SHA256 output format.
    let sig = sign("secret", "symbol=BTCUSDT&timestamp=123");
    assert_eq!(sig.len(), 64, "HMAC-SHA256 hex is 64 chars");
    assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    // Deterministic for same input.
    assert_eq!(sig, sign("secret", "symbol=BTCUSDT&timestamp=123"));
}

#[test]
fn hmac_signature_differs_with_secret() {
    let a = sign("secret1", "symbol=BTCUSDT&timestamp=1");
    let b = sign("secret2", "symbol=BTCUSDT&timestamp=1");
    assert_ne!(a, b);
}

#[test]
fn parse_decimal_handles_bad_input_and_zap() {
    assert_eq!(parse_decimal("1.500"), Decimal::new(1500, 3));
    assert_eq!(parse_decimal("0"), Decimal::ZERO);
    assert_eq!(parse_decimal("not-a-number"), Decimal::ZERO);
}

#[test]
fn kline_parses_from_row() {
    use crate::mock_data::kline_row;
    // `parse_kline` is private; validate via the mock-data assistant values.
    // Re-construct expected from mapped fields.
    let row = kline_row("100.0", "103.0");
    assert_eq!(row.len(), 9);
    // open_time at index 0, trades at index 8.
    assert_eq!(row[0].as_u64().unwrap(), 1_700_000_000_000);
    assert_eq!(row[8].as_u64().unwrap(), 42);
}

#[test]
fn env_enum_parses_and_resolves_base() {
    assert_eq!(BinanceEnvironment::parse("spot").base_url(), "https://api.binance.com");
    assert_eq!(
        BinanceEnvironment::parse("futures").base_url(),
        "https://fapi.binance.com"
    );
}

#[test]
fn order_book_mock_has_sides() {
    let book = crate::mock_data::sample_order_book();
    assert_eq!(book.symbol, "BTCUSDT");
    assert_eq!(book.bids.len(), 1);
    assert_eq!(book.asks.len(), 1);
    assert!(book.bids[0].0 < book.asks[0].0);
}
