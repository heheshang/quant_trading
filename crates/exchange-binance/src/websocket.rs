//! Binance WebSocket client.
//!
//! Independent Binance implementation: track subscriptions, connect to the
//! combined stream, reconnect on failure, and forward parsed messages through
//! an `mpsc` receiver.
//!
//! Protocol notes (spot):
//! - Connect: `wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1h`
//! - Dynamically subscribe: `{"method":"SUBSCRIBE","params":["..."],"id":N}`
//! - Incoming (combined): `{"stream":"<stream>","data":{...}}`

use futures::{SinkExt, StreamExt};
use quant_common::Result;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

use crate::types::{from_binance_symbol, BinanceEnvironment};

/// Message surfaced by the Binance WebSocket.
#[derive(Debug, Clone)]
pub enum BinanceWsMessage {
    ConnectionStatus(String),
    Kline(BinanceWsKline),
    Depth(BinanceWsDepth),
    /// Partial book depth (top-N snapshot) from `@depth20@100ms`.
    OrderBook(BinanceWsDepth),
    Ticker(BinanceWsTicker),
    Trade(BinanceWsTrade),
    AccountPosition(BinanceWsAccountPosition),
    OrderUpdate(BinanceWsOrderUpdate),
    Error(String),
}

/// A parsed kline from the stream.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsKline {
    pub symbol: String, // domain symbol, e.g. BTC-USDT
    pub interval: String,
    pub open_time: i64,
    pub open: rust_decimal::Decimal,
    pub high: rust_decimal::Decimal,
    pub low: rust_decimal::Decimal,
    pub close: rust_decimal::Decimal,
    pub volume: rust_decimal::Decimal,
    pub is_closed: bool,
}

/// A parsed depth update from the stream.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsDepth {
    pub symbol: String,
    pub bids: Vec<(rust_decimal::Decimal, rust_decimal::Decimal)>,
    pub asks: Vec<(rust_decimal::Decimal, rust_decimal::Decimal)>,
}

/// A parsed 24h ticker (`@ticker`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsTicker {
    pub symbol: String, // domain symbol, e.g. BTC-USDT
    pub last_price: rust_decimal::Decimal,
    pub price_change: rust_decimal::Decimal,
    pub price_change_percent: rust_decimal::Decimal,
    pub high: rust_decimal::Decimal,
    pub low: rust_decimal::Decimal,
    pub open: rust_decimal::Decimal,
    pub volume: rust_decimal::Decimal,
    pub quote_volume: rust_decimal::Decimal,
    pub event_time: i64,
}

/// A parsed trade (`@trade`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsTrade {
    pub symbol: String, // domain symbol, e.g. BTC-USDT
    pub price: rust_decimal::Decimal,
    pub quantity: rust_decimal::Decimal,
    pub trade_time: i64,
    /// `m` — true when the buyer is the maker (aggressor was a sell).
    pub is_buyer_maker: bool,
}
/// Parsed `outboundAccountPosition` (account balance update) from the user
/// data stream (`@userDataStream`). Pushed when balances change.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsAccountPosition {
    pub event_time: i64,
    pub balances: Vec<BinanceWsBalance>,
}

/// A single asset balance within [`BinanceWsAccountPosition`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsBalance {
    pub asset: String,
    pub free: rust_decimal::Decimal,
    pub locked: rust_decimal::Decimal,
}

/// Parsed `executionReport` (order update) from the user data stream.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsOrderUpdate {
    pub symbol: String, // domain symbol, e.g. BTC-USDT
    pub order_id: i64,
    pub client_order_id: String,
    pub side: String,         // BUY / SELL
    pub order_type: String,   // LIMIT / MARKET
    pub price: rust_decimal::Decimal,
    pub quantity: rust_decimal::Decimal,
    pub executed_quantity: rust_decimal::Decimal,
    pub status: String,       // NEW / FILLED / CANCELED / ...
    pub event_time: i64,
}

/// Binance WebSocket client.
pub struct BinanceWebSocket {
    environment: BinanceEnvironment,
    ws_url_override: Option<String>,
    streams: Arc<RwLock<Vec<String>>>,
    message_tx: mpsc::UnboundedSender<BinanceWsMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<BinanceWsMessage>>>,
    command_tx: broadcast::Sender<String>,
    shutdown_tx: watch::Sender<bool>,
}

impl BinanceWebSocket {
    pub fn new(environment: BinanceEnvironment, ws_url: Option<String>) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (command_tx, _) = broadcast::channel(256);
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            environment,
            ws_url_override: ws_url,
            streams: Arc::new(RwLock::new(Vec::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            command_tx,
            shutdown_tx,
        }
    }

    fn ws_url(&self) -> String {
        self.ws_url_override.clone().unwrap_or_else(|| match self.environment {
            BinanceEnvironment::Spot => "wss://stream.binance.com:9443/stream".to_string(),
            BinanceEnvironment::Futures => "wss://fstream.binance.com/stream".to_string(),
        })
    }

    /// Track a stream (e.g. `btcusdt@kline_1h`) and queue a subscribe command.
    pub async fn subscribe_stream(&self, stream: &str) -> Result<()> {
        let stream = stream.to_lowercase();
        let is_new = {
            let mut streams = self.streams.write().await;
            let set: HashSet<&String> = streams.iter().collect();
            if set.contains(&stream) {
                false
            } else {
                streams.push(stream.clone());
                true
            }
        };
        if is_new {
            let _ = self.command_tx.send(stream);
        }
        Ok(())
    }

    /// Subscribe to kline stream: `BTC-USDT` -> `btcusdt@kline_1h`.
    pub async fn subscribe_candle(&self, symbol: &str, interval: &str) -> Result<()> {
        let binance = crate::types::to_binance_symbol(symbol).to_lowercase();
        self.subscribe_stream(&format!("{}@kline_{}", binance, interval))
            .await
    }

    /// Subscribe to diff-depth stream: `BTC-USDT` -> `btcusdt@depth`.
    pub async fn subscribe_depth(&self, symbol: &str) -> Result<()> {
        let binance = crate::types::to_binance_symbol(symbol).to_lowercase();
        self.subscribe_stream(&format!("{}@depth", binance)).await
    }

    /// Subscribe to partial order-book snapshot: `BTC-USDT` -> `btcusdt@depth20@500ms`.
    ///
    /// The partial-book stream pushes a top-20 snapshot every update, so the
    /// consumer can render a full ladder without maintaining incremental deltas.
    /// @500ms 进一步降频(对比 @250ms 再减半)，对 20 档概览深度足够。
    pub async fn subscribe_orderbook(&self, symbol: &str) -> Result<()> {
        let binance = crate::types::to_binance_symbol(symbol).to_lowercase();
        self.subscribe_stream(&format!("{}@depth20@500ms", binance))
            .await
    }

    /// Subscribe to 24h ticker stream: `BTC-USDT` -> `btcusdt@ticker`.
    pub async fn subscribe_ticker(&self, symbol: &str) -> Result<()> {
        let binance = crate::types::to_binance_symbol(symbol).to_lowercase();
        self.subscribe_stream(&format!("{}@ticker", binance)).await
    }

    /// Subscribe to trades stream: `BTC-USDT` -> `btcusdt@trade`.
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let binance = crate::types::to_binance_symbol(symbol).to_lowercase();
        self.subscribe_stream(&format!("{}@trade", binance)).await
    }

    /// Subscribe to the account user-data stream via a `listenKey`.
    /// The key is a valid combined-stream name, so it is added as a stream.
    pub async fn subscribe_user_data(&self, listen_key: &str) -> Result<()> {
        self.subscribe_stream(&listen_key.to_lowercase()).await
    }

    pub async fn subscriptions(&self) -> Vec<String> {
        self.streams.read().await.clone()
    }

    /// Request the reconnect/read loop to stop.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Take an independent receiver for parsed messages.
    ///
    /// Swaps the stored receiver with a fresh one and returns the previous
    /// receiver, which keeps receiving all messages sent via the shared sender.
    pub async fn get_receiver(&self) -> mpsc::UnboundedReceiver<BinanceWsMessage> {
        let mut rx = self.message_rx.write().await;
        let (_new_tx, new_rx) = mpsc::unbounded_channel();
        std::mem::replace(&mut *rx, new_rx)
    }

    fn shutdown_requested(rx: &watch::Receiver<bool>) -> bool {
        *rx.borrow() || rx.has_changed().is_err()
    }

    /// Start the WebSocket (connect + auto-reconnect).
    pub async fn start(&self) -> Result<()> {
        let url = self.ws_url();
        let message_tx = self.message_tx.clone();
        let streams = self.streams.clone();
        let mut command_rx = self.command_tx.subscribe();
        let shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            let mut retry_delay = 1u64;
            loop {
                if Self::shutdown_requested(&shutdown_rx) {
                    info!("Binance WebSocket shutdown requested");
                    break;
                }
                let _ =
                    message_tx.send(BinanceWsMessage::ConnectionStatus("connecting".to_string()));
                let stream_query: Vec<String> = streams.read().await.clone();
                let conn_url = if stream_query.is_empty() {
                    url.clone()
                } else {
                    format!("{}?streams={}", url, stream_query.join("/"))
                };
                debug!("Connecting to Binance WebSocket: {}", conn_url);

                match connect_async(&conn_url).await {
                    Ok((ws_stream, _)) => {
                        let _ = message_tx
                            .send(BinanceWsMessage::ConnectionStatus("connected".to_string()));
                        retry_delay = 1;
                        let (mut write, mut read) = ws_stream.split();

                        // Send initial subscriptions (SUBSCRIBE ids).
                        let mut id = 1u64;
                        for stream in stream_query {
                            let sub = serde_json::json!({
                                "method": "SUBSCRIBE",
                                "params": [stream],
                                "id": id,
                            });
                            id += 1;
                            if write.send(Message::Text(sub.to_string())).await.is_err() {
                                break;
                            }
                        }

                        // Message loop.
                        loop {
                            if Self::shutdown_requested(&shutdown_rx) {
                                break;
                            }
                            tokio::select! {
                                cmd = command_rx.recv() => {
                                    match cmd {
                                        Ok(stream) => {
                                            let sub = serde_json::json!({
                                                "method": "SUBSCRIBE",
                                                "params": [stream],
                                                "id": id,
                                            });
                                            id += 1;
                                            let _ = write.send(Message::Text(sub.to_string())).await;
                                        }
                                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                                        Err(broadcast::error::RecvError::Closed) => break,
                                    }
                                }
                                msg = read.next() => {
                                    match msg {
                                        Some(Ok(Message::Text(text))) => {
                                            if let Some(parsed) = parse_ws_text(&text) {
                                                let _ = message_tx.send(parsed);
                                            }
                                        }
                                        Some(Ok(_)) => {}
                                        Some(Err(e)) => {
                                            let _ = message_tx.send(
                                                BinanceWsMessage::Error(format!("read error: {}", e)),
                                            );
                                            break;
                                        }
                                        None => break,
                                    }
                                }
                            }
                        }
                        info!("Binance WebSocket disconnected");
                    }
                    Err(e) => {
                        warn!("Binance WebSocket connect failed: {}", e);
                        let _ = message_tx
                            .send(BinanceWsMessage::Error(format!("connect failed: {}", e)));
                    }
                }

                tokio::time::sleep(Duration::from_secs(retry_delay)).await;
                retry_delay = (retry_delay * 2).min(30);
            }
        });

        Ok(())
    }
}

/// Parse an incoming text frame into a [`BinanceWsMessage`] (best-effort).
pub(crate) fn parse_ws_text(text: &str) -> Option<BinanceWsMessage> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        // Raw stream payload or combined stream envelope.
        if v.get("stream").is_some() && v.get("data").is_some() {
            let stream = v["stream"].as_str().unwrap_or_default().to_string();
            let data = v["data"].clone();
            return parse_data(&stream, &data);
        }
        // Direct (non-combined) payload.
        if v.get("data").is_some() {
            let stream = v["data"]["e"].as_str().unwrap_or_default().to_lowercase();
            return match stream.as_str() {
                "kline" => parse_kline(v["data"].clone()),
                "depthupdate" => parse_depth(v["data"].clone()),
                "24hrticker" => parse_ticker(v["data"].clone()),
                "trade" => parse_trade(v["data"].clone()),
                "outboundaccountposition" => parse_account_position(v["data"].clone()),
                "executionreport" => parse_order_update(v["data"].clone()),
                _ => None,
            };
        }
        // Bare event payload (`{"e": "outboundAccountPosition", ...}`) —
        // the WebSocket API delivers user-data events without a `data` wrapper.
        if let Some(e) = v.get("e").and_then(|x| x.as_str()) {
            return match e.to_lowercase().as_str() {
                "outboundaccountposition" => parse_account_position(v.clone()),
                "executionreport" => parse_order_update(v.clone()),
                _ => None,
            };
        }
        // Subscription ack.
        if v.get("result").is_some() {
            return None;
        }
    }
    None
}

fn parse_data(stream: &str, data: &serde_json::Value) -> Option<BinanceWsMessage> {
    if stream.contains("@kline") {
        parse_kline(data.clone())
    } else if stream.contains("@ticker") {
        parse_ticker(data.clone())
    } else if stream.contains("@trade") {
        parse_trade(data.clone())
    } else if stream.contains("@depth20") {
        parse_depth20(stream, data.clone())
    } else if stream.contains("@depth") {
        parse_depth(data.clone())
    } else {
        None
    }
}

fn parse_kline(data: serde_json::Value) -> Option<BinanceWsMessage> {
    let k = data.get("k")?;
    let symbol = data.get("s")?.as_str()?;
    let interval = k.get("i")?.as_str()?;
    Some(BinanceWsMessage::Kline(BinanceWsKline {
        symbol: from_binance_symbol(symbol),
        interval: interval.to_string(),
        open_time: k.get("t")?.as_i64()?,
        open: parse_decimal_str(k.get("o")?.as_str()?),
        high: parse_decimal_str(k.get("h")?.as_str()?),
        low: parse_decimal_str(k.get("l")?.as_str()?),
        close: parse_decimal_str(k.get("c")?.as_str()?),
        volume: parse_decimal_str(k.get("v")?.as_str()?),
        is_closed: k.get("x")?.as_bool().unwrap_or(false),
    }))
}

fn parse_depth(data: serde_json::Value) -> Option<BinanceWsMessage> {
    let symbol = data.get("s")?.as_str()?;
    let bids = parse_levels(data.get("b")?);
    let asks = parse_levels(data.get("a")?);
    Some(BinanceWsMessage::Depth(BinanceWsDepth {
        symbol: from_binance_symbol(symbol),
        bids,
        asks,
    }))
}

/// Parse a partial-book depth snapshot (`@depth20@100ms`). The payload has no
/// symbol field, so it is derived from the combined-stream name.
fn parse_depth20(stream: &str, data: serde_json::Value) -> Option<BinanceWsMessage> {
    let symbol = stream.split("@depth").next()?;
    let bids = parse_levels(data.get("bids")?);
    let asks = parse_levels(data.get("asks")?);
    Some(BinanceWsMessage::OrderBook(BinanceWsDepth {
        symbol: from_binance_symbol(&symbol.to_uppercase()),
        bids,
        asks,
    }))
}

/// Parse a 24h ticker (`@ticker`).
fn parse_ticker(data: serde_json::Value) -> Option<BinanceWsMessage> {
    let symbol = data.get("s")?.as_str()?;
    Some(BinanceWsMessage::Ticker(BinanceWsTicker {
        symbol: from_binance_symbol(symbol),
        last_price: parse_decimal_str(data.get("c")?.as_str()?),
        price_change: parse_decimal_str(data.get("p")?.as_str()?),
        price_change_percent: parse_decimal_str(data.get("P")?.as_str()?),
        high: parse_decimal_str(data.get("h")?.as_str()?),
        low: parse_decimal_str(data.get("l")?.as_str()?),
        open: parse_decimal_str(data.get("o")?.as_str()?),
        volume: parse_decimal_str(data.get("v")?.as_str()?),
        quote_volume: parse_decimal_str(data.get("q")?.as_str()?),
        event_time: data.get("E")?.as_i64().unwrap_or(0),
    }))
}

/// Parse a trade (`@trade`).
fn parse_trade(data: serde_json::Value) -> Option<BinanceWsMessage> {
    let symbol = data.get("s")?.as_str()?;
    Some(BinanceWsMessage::Trade(BinanceWsTrade {
        symbol: from_binance_symbol(symbol),
        price: parse_decimal_str(data.get("p")?.as_str()?),
        quantity: parse_decimal_str(data.get("q")?.as_str()?),
        trade_time: data.get("T")?.as_i64()?,
        is_buyer_maker: data.get("m")?.as_bool().unwrap_or(false),
    }))
}

fn parse_account_position(data: serde_json::Value) -> Option<BinanceWsMessage> {
    let balances = data
        .get("B")?
        .as_array()?
        .iter()
        .filter_map(|b| {
            let asset = b.get("a")?.as_str()?.to_string();
            let free = parse_decimal_str(b.get("f")?.as_str()?);
            let locked = parse_decimal_str(b.get("l")?.as_str()?);
            Some(BinanceWsBalance { asset, free, locked })
        })
        .collect();
    Some(BinanceWsMessage::AccountPosition(BinanceWsAccountPosition {
        event_time: data.get("E")?.as_i64()?,
        balances,
    }))
}

fn parse_order_update(data: serde_json::Value) -> Option<BinanceWsMessage> {
    Some(BinanceWsMessage::OrderUpdate(BinanceWsOrderUpdate {
        symbol: from_binance_symbol(data.get("s")?.as_str()?),
        order_id: data.get("i")?.as_i64()?,
        client_order_id: data.get("c")?.as_str().unwrap_or_default().to_string(),
        side: data.get("S")?.as_str().unwrap_or_default().to_string(),
        order_type: data.get("o")?.as_str().unwrap_or_default().to_string(),
        price: parse_decimal_str(data.get("p")?.as_str()?),
        quantity: parse_decimal_str(data.get("q")?.as_str()?),
        executed_quantity: parse_decimal_str(data.get("z")?.as_str()?),
        status: data.get("X")?.as_str().unwrap_or_default().to_string(),
        event_time: data.get("E")?.as_i64()?,
    }))
}

fn parse_levels(arr: &serde_json::Value) -> Vec<(rust_decimal::Decimal, rust_decimal::Decimal)> {
    arr.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| {
                    let price = r
                        .get(0)?
                        .as_str()?
                        .parse::<rust_decimal::Decimal>()
                        .unwrap_or(rust_decimal::Decimal::ZERO);
                    let qty = r
                        .get(1)?
                        .as_str()?
                        .parse::<rust_decimal::Decimal>()
                        .unwrap_or(rust_decimal::Decimal::ZERO);
                    Some((price, qty))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_decimal_str(s: &str) -> rust_decimal::Decimal {
    s.parse::<rust_decimal::Decimal>()
        .unwrap_or(rust_decimal::Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_combined_kline_envelope() {
        let msg = r#"{"stream":"btcusdt@kline_1h","data":{"e":"kline","E":1,"s":"BTCUSDT","k":{"t":2,"o":"100","h":"105","l":"95","c":"103","v":"1200","x":true,"i":"1h","n":10}}}"#;
        let parsed = parse_ws_text(msg).expect("parse");
        match parsed {
            BinanceWsMessage::Kline(k) => {
                assert_eq!(k.symbol, "BTC-USDT");
                assert_eq!(k.interval, "1h");
                assert!(k.is_closed);
            }
            _ => panic!("expected kline"),
        }
    }

    #[test]
    fn parses_depth_payload() {
        let msg = r#"{"stream":"btcusdt@depth","data":{"e":"depthUpdate","E":1,"s":"BTCUSDT","b":[["100","1"]],"a":[["101","2"]]}}"#;
        let parsed = parse_ws_text(msg).expect("parse");
        match parsed {
            BinanceWsMessage::Depth(d) => {
                assert_eq!(d.symbol, "BTC-USDT");
                assert_eq!(d.bids.len(), 1);
                assert_eq!(d.asks.len(), 1);
            }
            _ => panic!("expected depth"),
        }
    }

    #[test]
    fn ignores_subscribe_ack() {
        assert!(parse_ws_text(r#"{"result":null,"id":1}"#).is_none());
    }

    #[test]
    fn parses_ticker_payload() {
        let msg = r#"{"stream":"btcusdt@ticker","data":{"e":"24hrTicker","E":1700000000000,"s":"BTCUSDT","p":"10.00","P":"1.20","h":"900","l":"800","o":"830","c":"840","v":"1000","q":"840000"}}"#;
        let parsed = parse_ws_text(msg).expect("parse");
        match parsed {
            BinanceWsMessage::Ticker(t) => {
                assert_eq!(t.symbol, "BTC-USDT");
                assert_eq!(t.last_price, rust_decimal::Decimal::new(840, 0));
                assert_eq!(t.price_change, rust_decimal::Decimal::new(10, 0));
                assert_eq!(t.price_change_percent, rust_decimal::Decimal::new(120, 2));
                assert_eq!(t.high, rust_decimal::Decimal::new(900, 0));
                assert_eq!(t.low, rust_decimal::Decimal::new(800, 0));
                assert_eq!(t.volume, rust_decimal::Decimal::new(1000, 0));
                assert_eq!(t.event_time, 1_700_000_000_000);
            }
            _ => panic!("expected ticker"),
        }
    }

    #[test]
    fn parses_trade_payload() {
        let msg = r#"{"stream":"btcusdt@trade","data":{"e":"trade","E":1700000000000,"s":"BTCUSDT","t":42,"p":"840.10","q":"0.5","T":1700000000100,"m":true,"M":true}}"#;
        let parsed = parse_ws_text(msg).expect("parse");
        match parsed {
            BinanceWsMessage::Trade(t) => {
                assert_eq!(t.symbol, "BTC-USDT");
                assert_eq!(t.price, rust_decimal::Decimal::new(84010, 2));
                assert_eq!(t.quantity, rust_decimal::Decimal::new(5, 1));
                assert_eq!(t.trade_time, 1_700_000_000_100);
                assert!(t.is_buyer_maker);
            }
            _ => panic!("expected trade"),
        }
    }

    #[test]
    fn parses_partial_book_depth_payload() {
        let msg = r#"{"stream":"btcusdt@depth20@100ms","data":{"lastUpdateId":160,"bids":[["100.00","1.5"]],"asks":[["101.00","2.5"]]}}"#;
        let parsed = parse_ws_text(msg).expect("parse");
        match parsed {
            BinanceWsMessage::OrderBook(d) => {
                assert_eq!(d.symbol, "BTC-USDT");
                assert_eq!(d.bids.len(), 1);
                assert_eq!(
                    d.bids[0],
                    (
                        rust_decimal::Decimal::new(10000, 2),
                        rust_decimal::Decimal::new(15, 1)
                    )
                );
                assert_eq!(d.asks.len(), 1);
                assert_eq!(
                    d.asks[0],
                    (
                        rust_decimal::Decimal::new(10100, 2),
                        rust_decimal::Decimal::new(25, 1)
                    )
                );
            }
            _ => panic!("expected orderbook"),
        }
    }

    #[test]
    fn symbol_stream_format() {
        assert_eq!(
            crate::types::to_binance_symbol("BTC-USDT").to_lowercase(),
            "btcusdt"
        );
    }
}
