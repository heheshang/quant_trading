//! Binance exchange client.
//!
//! Independent Binance implementation: a trait-based REST client (`ClientInterface`)
//! with a `reqwest` + HMAC-SHA256 implementation (`Client`), plus shared types and
//! symbol-conversion helpers.

pub mod client;
pub mod types;
pub mod user_data_stream;
pub mod websocket;

pub use client::{Client, ClientInterface};
pub use types::{
    from_binance_symbol, to_binance_symbol, BinanceBalance, BinanceEnvironment, BinanceKline,
    BinanceOrder, BinanceOrderBook, BinanceOrderType, BinancePlaceOrderRequest, BinancePosition,
    BinanceSide, BinanceTicker24h,
};
pub use user_data_stream::UserDataStreamClient;
pub use websocket::{BinanceWebSocket, BinanceWsMessage};

#[cfg(test)]
pub mod mock_data;
#[cfg(any(test, feature = "test-utils"))]
pub use client::MockClientInterface as MockBinanceClient;

#[cfg(test)]
mod tests;
