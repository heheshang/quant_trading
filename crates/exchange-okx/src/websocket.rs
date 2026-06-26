use crate::types::*;
use futures::{SinkExt, StreamExt};
use quant_common::{Error, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// WebSocket 消息类型
#[derive(Debug, Clone)]
pub enum WsMessage {
    Ticker(serde_json::Value),
    Trades(serde_json::Value),
    OrderBook(serde_json::Value),
    Candle(serde_json::Value),
    Account(serde_json::Value),
    Orders(serde_json::Value),
    Positions(serde_json::Value),
    Error(String),
}

/// OKX WebSocket 客户端
pub struct OkxWebSocket {
    environment: OkxEnvironment,
    subscriptions: Arc<RwLock<Vec<OkxWsSubscription>>>,
    message_tx: mpsc::UnboundedSender<WsMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<WsMessage>>>,
}

impl OkxWebSocket {
    /// 创建新的 WebSocket 客户端
    pub fn new(environment: OkxEnvironment) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        Self {
            environment,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
        }
    }

    /// 订阅公共频道
    pub async fn subscribe_public(&self, channel: &str, inst_id: &str) -> Result<()> {
        let subscription = OkxWsSubscription {
            channel: channel.to_string(),
            inst_id: inst_id.to_string(),
        };

        let mut subs = self.subscriptions.write().await;
        subs.push(subscription);

        Ok(())
    }

    /// 订阅 Ticker
    pub async fn subscribe_ticker(&self, inst_id: &str) -> Result<()> {
        self.subscribe_public("tickers", inst_id).await
    }

    /// 订阅交易数据
    pub async fn subscribe_trades(&self, inst_id: &str) -> Result<()> {
        self.subscribe_public("trades", inst_id).await
    }

    /// 订阅订单簿
    pub async fn subscribe_order_book(&self, inst_id: &str, depth: &str) -> Result<()> {
        let channel = format!("books{}", depth); // books5, books-l2-tbt
        self.subscribe_public(&channel, inst_id).await
    }

    /// 订阅 K 线
    pub async fn subscribe_candle(&self, inst_id: &str, bar: &str) -> Result<()> {
        let channel = format!("candle{}", bar); // candle1m, candle5m
        self.subscribe_public(&channel, inst_id).await
    }

    /// 获取当前订阅列表
    pub async fn subscriptions(&self) -> Vec<OkxWsSubscription> {
        self.subscriptions.read().await.clone()
    }

    /// 启动 WebSocket 连接
    pub async fn start(&self) -> Result<()> {
        let url = self.environment.ws_public_url();
        info!("Connecting to OKX WebSocket: {}", url);

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| Error::Network(format!("WebSocket connection failed: {}", e)))?;

        info!("WebSocket connected to {}", url);

        let (mut write, mut read) = ws_stream.split();
        let message_tx = self.message_tx.clone();
        let subscriptions = self.subscriptions.clone();

        // 发送订阅消息
        let subs = subscriptions.read().await.clone();
        if !subs.is_empty() {
            let subscribe_msg = serde_json::json!({
                "op": "subscribe",
                "args": subs
            });

            let msg_str = serde_json::to_string(&subscribe_msg)
                .map_err(|e| Error::Internal(format!("Serialize subscribe msg failed: {}", e)))?;

            debug!("Sending subscription: {}", msg_str);

            write
                .send(Message::Text(msg_str))
                .await
                .map_err(|e| Error::Network(format!("Send subscribe failed: {}", e)))?;
        }

        // 启动心跳任务
        let message_tx_clone = message_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(25));
            loop {
                interval.tick().await;
                let _ = message_tx_clone.send(WsMessage::Error("ping".to_string()));
                debug!("Heartbeat sent");
            }
        });

        // 读取消息
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        debug!("Received message: {}", text);

                        if text == "pong" {
                            debug!("Received pong");
                            continue;
                        }

                        match serde_json::from_str::<OkxWsMessage>(&text) {
                            Ok(msg) => {
                                // 处理不同类型的消息
                                if let Some(ref event) = msg.event {
                                    match event.as_str() {
                                        "subscribe" => {
                                            info!("Subscription confirmed");
                                        }
                                        "error" => {
                                            let msg_clone = format!("{:?}", msg);
                                            error!("Subscription error: {}", msg_clone);
                                            let _ = message_tx.send(WsMessage::Error(
                                                msg.msg.unwrap_or_default(),
                                            ));
                                        }
                                        _ => {}
                                    }
                                } else if let Some(arg) = &msg.arg {
                                    if let Some(data) = msg.data {
                                        // 根据频道类型分发消息
                                        let ws_msg = if arg.channel.starts_with("tickers") {
                                            WsMessage::Ticker(data)
                                        } else if arg.channel.starts_with("trades") {
                                            WsMessage::Trades(data)
                                        } else if arg.channel.starts_with("books") {
                                            WsMessage::OrderBook(data)
                                        } else if arg.channel.starts_with("candle") {
                                            WsMessage::Candle(data)
                                        } else {
                                            continue;
                                        };

                                        let _ = message_tx.send(ws_msg);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse message: {}, text: {}", e, text);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        warn!("WebSocket disconnected");
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        let _ = message_tx.send(WsMessage::Error(e.to_string()));
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// 接收消息
    pub async fn receive(&self) -> Option<WsMessage> {
        let mut rx = self.message_rx.write().await;
        rx.recv().await
    }

    /// 获取消息接收器
    pub async fn get_receiver(&self) -> mpsc::UnboundedReceiver<WsMessage> {
        let mut rx = self.message_rx.write().await;
        let (_new_tx, new_rx) = mpsc::unbounded_channel();

        // 替换旧的接收器
        let old_rx = std::mem::replace(&mut *rx, new_rx);

        old_rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_creation() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        assert!(ws.subscribe_ticker("BTC-USDT").await.is_ok());
    }
}
