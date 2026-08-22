//! Binance WebSocket client.
//!
//! Mirrors `exchange-okx::websocket::OkxWebSocket`'s public surface: track
//! subscriptions, connect to the combined stream, reconnect on failure, and
//! forward parsed messages through an `mpsc` receiver.
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

use crate::types::{BinanceEnvironment, from_binance_symbol};

/// Message surfaced by the Binance WebSocket.
#[derive(Debug, Clone)]
pub enum BinanceWsMessage {
    ConnectionStatus(String),
    Kline(BinanceWsKline),
    Depth(BinanceWsDepth),
    Error(String),
}

/// A parsed kline from the stream.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BinanceWsKline {
    pub symbol: String,     // domain symbol, e.g. BTC-USDT
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

/// Binance WebSocket client.
pub struct BinanceWebSocket {
    environment: BinanceEnvironment,
    streams: Arc<RwLock<Vec<String>>>,
    message_tx: mpsc::UnboundedSender<BinanceWsMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<BinanceWsMessage>>>,
    command_tx: broadcast::Sender<String>,
    shutdown_tx: watch::Sender<bool>,
}

impl BinanceWebSocket {
    pub fn new(environment: BinanceEnvironment) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (command_tx, _) = broadcast::channel(256);
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            environment,
            streams: Arc::new(RwLock::new(Vec::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            command_tx,
            shutdown_tx,
        }
    }

    fn ws_url(&self) -> String {
        match self.environment {
            BinanceEnvironment::Spot => {
                "wss://stream.binance.com:9443/stream".to_string()
            }
            BinanceEnvironment::Futures => "wss://fstream.binance.com/stream".to_string(),
        }
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

    /// Subscribe to depth stream: `BTC-USDT` -> `btcusdt@depth`.
    pub async fn subscribe_depth(&self, symbol: &str) -> Result<()> {
        let binance = crate::types::to_binance_symbol(symbol).to_lowercase();
        self.subscribe_stream(&format!("{}@depth", binance)).await
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
                let _ = message_tx.send(BinanceWsMessage::ConnectionStatus("connecting".to_string()));
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
                        let _ = message_tx.send(BinanceWsMessage::Error(format!(
                            "connect failed: {}",
                            e
                        )));
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
fn parse_ws_text(text: &str) -> Option<BinanceWsMessage> {
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

fn parse_levels(arr: &serde_json::Value) -> Vec<(rust_decimal::Decimal, rust_decimal::Decimal)> {
    arr.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| {
                    let price = r.get(0)?.as_str()?.parse::<rust_decimal::Decimal>()
                        .unwrap_or(rust_decimal::Decimal::ZERO);
                    let qty = r.get(1)?.as_str()?.parse::<rust_decimal::Decimal>()
                        .unwrap_or(rust_decimal::Decimal::ZERO);
                    Some((price, qty))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_decimal_str(s: &str) -> rust_decimal::Decimal {
    s.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO)
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
    fn symbol_stream_format() {
        assert_eq!(
            crate::types::to_binance_symbol("BTC-USDT").to_lowercase(),
            "btcusdt"
        );
    }
}
