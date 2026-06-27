use crate::types::*;
use futures::{SinkExt, StreamExt};
use quant_common::{Error, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
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
    ConnectionStatus(String), // "connecting", "connected", "disconnected"
    Error(String),
}

/// 内部 WS 命令：从外部发送给连接循环
type WsCommand = (String, String); // (channel, inst_id)

/// OKX WebSocket 客户端
pub struct OkxWebSocket {
    environment: OkxEnvironment,
    subscriptions: Arc<RwLock<Vec<OkxWsSubscription>>>,
    message_tx: mpsc::UnboundedSender<WsMessage>,
    message_rx: Arc<RwLock<mpsc::UnboundedReceiver<WsMessage>>>,
    command_tx: broadcast::Sender<WsCommand>,
}

impl OkxWebSocket {
    /// 创建新的 WebSocket 客户端
    pub fn new(environment: OkxEnvironment) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (command_tx, _) = broadcast::channel(256);

        Self {
            environment,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(message_rx)),
            command_tx,
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

    /// 发送订阅到已连接的 WS 服务器 (通过 broadcast 信道)
    pub async fn send_subscription(&self, channel: &str, inst_id: &str) -> Result<()> {
        self.command_tx
            .send((channel.to_string(), inst_id.to_string()))
            .map_err(|e| Error::Internal(format!("Failed to queue subscription: {}", e)))?;
        Ok(())
    }

    /// 处理单条 WS 消息并返回是否应该断开
    async fn handle_ws_message(
        message: std::result::Result<Message, tokio_tungstenite::tungstenite::Error>,
        message_tx: &mpsc::UnboundedSender<WsMessage>,
    ) -> bool {
        match message {
            Ok(Message::Text(text)) => {
                debug!("Received message: {}", text);

                match serde_json::from_str::<OkxWsMessage>(&text) {
                    Ok(msg) => {
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
                                let ws_msg = if arg.channel.starts_with("tickers") {
                                    WsMessage::Ticker(data)
                                } else if arg.channel.starts_with("trades") {
                                    WsMessage::Trades(data)
                                } else if arg.channel.starts_with("books") {
                                    WsMessage::OrderBook(data)
                                } else if arg.channel.starts_with("candle") {
                                    WsMessage::Candle(data)
                                } else {
                                    return false;
                                };
                                let _ = message_tx.send(ws_msg);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse message: {}, text: {}", e, text);
                    }
                }
                false // no disconnect
            }
            Ok(Message::Close(_)) => {
                warn!("WebSocket disconnected");
                true // disconnect
            }
            Ok(Message::Ping(_)) => {
                // OKX 发送 ping, 自动回复 pong 由 tungstenite 处理
                false
            }
            Ok(Message::Pong(_)) => {
                debug!("Received pong");
                false
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                true // disconnect
            }
            _ => false,
        }
    }

    /// 启动 WebSocket 连接（含自动重连 + broadcast 订阅信道）
    pub async fn start(&self) -> Result<()> {
        let url = self.environment.ws_public_url().to_string();
        let message_tx = self.message_tx.clone();
        let subscriptions = self.subscriptions.clone();
        let mut command_rx = self.command_tx.subscribe();

        // 指数退避重连循环 (后台任务)
        tokio::spawn(async move {
            let mut retry_delay = 1u64;

            loop {
                let _ = message_tx.send(WsMessage::ConnectionStatus("connecting".to_string()));
                info!("Connecting to OKX WebSocket (delay={}s): {}", retry_delay, url);

                match connect_async(url.clone()).await {
                    Ok((ws_stream, _)) => {
                        info!("WebSocket connected");
                        let _ = message_tx.send(WsMessage::ConnectionStatus("connected".to_string()));
                        retry_delay = 1;

                        let (mut write, mut read) = ws_stream.split();
                        let subs = subscriptions.read().await.clone();

                        // 发送初始订阅
                        if !subs.is_empty() {
                            let subscribe_msg = serde_json::json!({"op": "subscribe", "args": subs});
                            if let Ok(msg_str) = serde_json::to_string(&subscribe_msg) {
                                let _ = write.send(Message::Text(msg_str)).await;
                            }
                        }

                        // 读循环: WS 消息 + new subscriptions + 心跳
                        loop {
                            tokio::select! {
                                biased; // 优先处理 WS 消息

                                msg_result = read.next() => {
                                    match msg_result {
                                        Some(msg) => {
                                            let should_disconnect = Self::handle_ws_message(
                                                msg, &message_tx,
                                            ).await;
                                            if should_disconnect {
                                                let _ = message_tx.send(
                                                    WsMessage::ConnectionStatus("disconnected".to_string())
                                                );
                                                break;
                                            }
                                        }
                                        None => {
                                            warn!("WebSocket read stream ended");
                                            let _ = message_tx.send(
                                                WsMessage::ConnectionStatus("disconnected".to_string())
                                            );
                                            break;
                                        }
                                    }
                                }

                                // 20s 心跳: 发送 OKX 可识别的 ping 请求
                                _ = tokio::time::sleep(Duration::from_secs(20)) => {
                                    let ping = serde_json::json!({"op": "ping"});
                                    if let Ok(ping_str) = serde_json::to_string(&ping) {
                                        let _ = write.send(Message::Text(ping_str)).await;
                                    }
                                }

                                // 处理 broadcast 命令 (新订阅)
                                Ok(cmd) = command_rx.recv() => {
                                    let (channel, inst_id) = cmd;
                                    let sub = OkxWsSubscription {
                                        channel: channel.clone(),
                                        inst_id: inst_id.clone(),
                                    };
                                    let subscribe_msg = serde_json::json!({
                                        "op": "subscribe",
                                        "args": [sub]
                                    });
                                    if let Ok(msg_str) = serde_json::to_string(&subscribe_msg) {
                                        debug!("Sending new subscription: {}", msg_str);
                                        let _ = write.send(Message::Text(msg_str)).await;
                                    }
                                }

                                else => {
                                    // 所有信道关闭，退出重连
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("WebSocket connection failed: {}", e);
                        let _ = message_tx.send(WsMessage::Error(format!("Connection failed: {}", e)));
                    }
                }

                // 指数退避
                tokio::time::sleep(Duration::from_secs(retry_delay)).await;
                retry_delay = std::cmp::min(retry_delay * 2, 30);
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
    async fn test_initial_state() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        let subs = ws.subscriptions().await;
        assert!(subs.is_empty(), "new WebSocket should have no subscriptions");
    }

    #[tokio::test]
    async fn test_new_environment() {
        let ws_demo = OkxWebSocket::new(OkxEnvironment::Demo);
        let ws_live = OkxWebSocket::new(OkxEnvironment::Live);
        assert_eq!(ws_demo.environment, OkxEnvironment::Demo);
        assert_eq!(ws_live.environment, OkxEnvironment::Live);
    }

    #[tokio::test]
    async fn test_subscribe_ticker_adds_to_list() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        ws.subscribe_ticker("BTC-USDT").await.unwrap();

        let subs = ws.subscriptions().await;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel, "tickers");
        assert_eq!(subs[0].inst_id, "BTC-USDT");
    }

    #[tokio::test]
    async fn test_subscribe_public_creates_correct_subscription() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        ws.subscribe_public("trades", "ETH-USDT").await.unwrap();

        let subs = ws.subscriptions().await;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel, "trades");
        assert_eq!(subs[0].inst_id, "ETH-USDT");
    }

    #[tokio::test]
    async fn test_subscribe_trades() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        ws.subscribe_trades("SOL-USDT").await.unwrap();

        let subs = ws.subscriptions().await;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel, "trades");
        assert_eq!(subs[0].inst_id, "SOL-USDT");
    }

    #[tokio::test]
    async fn test_subscribe_order_book() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        ws.subscribe_order_book("BTC-USDT", "5").await.unwrap();

        let subs = ws.subscriptions().await;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel, "books5");
        assert_eq!(subs[0].inst_id, "BTC-USDT");
    }

    #[tokio::test]
    async fn test_subscribe_candle() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        ws.subscribe_candle("BTC-USDT", "1m").await.unwrap();

        let subs = ws.subscriptions().await;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel, "candle1m");
        assert_eq!(subs[0].inst_id, "BTC-USDT");
    }

    #[tokio::test]
    async fn test_subscribe_multiple_instruments() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        ws.subscribe_ticker("BTC-USDT").await.unwrap();
        ws.subscribe_ticker("ETH-USDT").await.unwrap();
        ws.subscribe_ticker("SOL-USDT").await.unwrap();

        let subs = ws.subscriptions().await;
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0].inst_id, "BTC-USDT");
        assert_eq!(subs[1].inst_id, "ETH-USDT");
        assert_eq!(subs[2].inst_id, "SOL-USDT");
    }

    #[tokio::test]
    async fn test_subscribe_multiple_channels() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        ws.subscribe_ticker("BTC-USDT").await.unwrap();
        ws.subscribe_trades("BTC-USDT").await.unwrap();
        ws.subscribe_order_book("BTC-USDT", "5").await.unwrap();

        let subs = ws.subscriptions().await;
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0].channel, "tickers");
        assert_eq!(subs[1].channel, "trades");
        assert_eq!(subs[2].channel, "books5");
    }

    #[tokio::test]
    async fn test_websocket_creation() {
        let ws = OkxWebSocket::new(OkxEnvironment::Demo);
        assert!(ws.subscribe_ticker("BTC-USDT").await.is_ok());
    }
}
