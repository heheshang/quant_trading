//! Binance exchange client.
//!
//! Mirrors `exchange-okx`: a trait-based REST client (`ClientInterface`) with
//! a `reqwest` + HMAC-SHA256 implementation (`Client`), plus shared types and
//! symbol-conversion helpers.

pub mod client;
pub mod types;
pub mod websocket;

pub use client::{Client, ClientInterface};
pub use websocket::{BinanceWebSocket, BinanceWsMessage};
pub use types::{
    BinanceBalance, BinanceEnvironment, BinanceKline, BinanceOrder, BinanceOrderBook,
    BinanceOrderType, BinancePlaceOrderRequest, BinanceSide, from_binance_symbol,
    to_binance_symbol,
};

#[cfg(test)]
pub mod mock_data;
#[cfg(any(test, feature = "test-utils"))]
pub use client::MockClientInterface as MockBinanceClient;

#[cfg(test)]
mod tests;
