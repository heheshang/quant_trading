//! Binance WebSocket-API user data stream client.
//!
//! Uses the WebSocket API (`wss://ws-api.binance.com/ws-api/v3`) — the
//! successor to the deprecated REST `userDataStream` listenKey endpoint, which
//! now returns `410 Gone`. Flow: connect → `userDataStream.start` → listenKey →
//! `userDataStream.subscribe` → receive `outboundAccountPosition` /
//! `executionReport` → keepalive every 30 min → `userDataStream.stop`.

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use quant_common::{Error, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, info, warn};

use crate::websocket::{parse_ws_text, BinanceWsMessage};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsWrite = SplitSink<WsStream, Message>;
type WsRead = SplitStream<WsStream>;

const KEEPALIVE_INTERVAL_SECS: u64 = 30 * 60;

/// Client for the Binance WebSocket-API user data stream.
pub struct UserDataStreamClient {
    url: String,
    api_key: String,
    message_tx: mpsc::UnboundedSender<BinanceWsMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<BinanceWsMessage>>>,
    shutdown_tx: watch::Sender<bool>,
}

impl UserDataStreamClient {
    pub fn new(ws_api_url: String, api_key: String) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            url: ws_api_url,
            api_key,
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            shutdown_tx,
        }
    }

    /// Take an independent receiver for parsed messages.
    pub async fn get_receiver(&self) -> mpsc::UnboundedReceiver<BinanceWsMessage> {
        let mut rx = self.message_rx.write().await;
        let (_tx, new_rx) = mpsc::unbounded_channel();
        std::mem::replace(&mut *rx, new_rx)
    }

    /// Request the read loop to stop.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Connect, `userDataStream.start` → listenKey, then subscribe. Spawns the
    /// read + keepalive loop. Returns the listenKey.
    pub async fn start(&self) -> Result<String> {
        let (ws, _) = connect_async(&self.url)
            .await
            .map_err(|e| Error::Network(format!("userDataStream connect failed: {e}")))?;
        info!("Connected to user data stream: {}", self.url);
        let (mut write, mut read) = ws.split();

        // 1. userDataStream.start → listenKey.
        write
            .send(Message::Text(
                serde_json::json!({"id": 1, "method": "userDataStream.start", "params": {"apiKey": self.api_key}})
                    .to_string(),
            ))
            .await
            .map_err(|e| Error::Network(format!("userDataStream.start send failed: {e}")))?;
        let listen_key = await_listen_key(&mut read).await?;

        // 2. Subscribe.
        write
            .send(Message::Text(
                serde_json::json!({
                    "id": 2,
                    "method": "userDataStream.subscribe",
                    "params": { "listenKey": listen_key },
                })
                .to_string(),
            ))
            .await
            .map_err(|e| Error::Network(format!("userDataStream.subscribe send failed: {e}")))?;

        // 3. Spawn read + keepalive loop.
        let message_tx = self.message_tx.clone();
        let shutdown_rx = self.shutdown_tx.subscribe();
        let lk = listen_key.clone();
        tokio::spawn(async move {
            run_loop(&mut write, &mut read, &message_tx, &lk, shutdown_rx).await;
        });

        Ok(listen_key)
    }
}

/// Read until the `userDataStream.start` response containing the listenKey.
async fn await_listen_key(read: &mut WsRead) -> Result<String> {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let v: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if v.get("id").and_then(|i| i.as_u64()) == Some(1) {
                    if let Some(key) = v.pointer("/result/listenKey").and_then(|k| k.as_str()) {
                        return Ok(key.to_string());
                    }
                    if let Some(err) = v.get("error") {
                        return Err(Error::Network(format!(
                            "userDataStream.start failed: {err}"
                        )));
                    }
                }
            }
            Ok(_) => {}
            Err(e) => return Err(Error::Network(format!("userDataStream read error: {e}"))),
        }
    }
    Err(Error::Network("userDataStream.start timed out".into()))
}

/// Read loop: forward user-data events + keepalive every 30 min.
async fn run_loop(
    write: &mut WsWrite,
    read: &mut WsRead,
    message_tx: &mpsc::UnboundedSender<BinanceWsMessage>,
    listen_key: &str,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut next_keepalive =
        tokio::time::Instant::now() + Duration::from_secs(KEEPALIVE_INTERVAL_SECS);
    loop {
        if *shutdown_rx.borrow() {
            info!("User data stream shutdown requested");
            break;
        }
        tokio::select! {
            _ = shutdown_rx.changed() => break,
            _ = tokio::time::sleep_until(next_keepalive) => {
                next_keepalive = tokio::time::Instant::now() + Duration::from_secs(KEEPALIVE_INTERVAL_SECS);
                let req = serde_json::json!({
                    "id": 3,
                    "method": "userDataStream.keepalive",
                    "params": { "listenKey": listen_key },
                }).to_string();
                if let Err(e) = write.send(Message::Text(req)).await {
                    warn!(error = %e, "userDataStream keepalive send failed");
                    break;
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
                        let _ = message_tx.send(BinanceWsMessage::Error(format!(
                            "userDataStream read error: {e}"
                        )));
                        break;
                    }
                    None => break,
                }
            }
        }
    }
    debug!("User data stream loop ended");
}
