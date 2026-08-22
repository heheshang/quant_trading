//! WebSocket 消息类型与解析。

use crate::types::*;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
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
#[derive(Debug, Clone)]
pub(super) enum WsCommand {
    Subscribe { channel: String, inst_id: String },
    Unsubscribe { channel: String, inst_id: String },
}

/// 处理单条 WS 消息并返回是否应该断开
pub(super) async fn handle_ws_message(
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
                                let _ =
                                    message_tx.send(WsMessage::Error(msg.msg.unwrap_or_default()));
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
