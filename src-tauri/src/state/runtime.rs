//! 运行时状态模块
//!
//! 包含 WebSocket 状态等运行时相关信息。

use exchange_binance::{websocket::BinanceWebSocket, UserDataStreamClient};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Binance WebSocket 连接状态
pub struct BinanceWsState {
    pub running: Arc<AtomicBool>,
    pub ws: RwLock<Option<BinanceWebSocket>>,
    pub user_data_running: Arc<AtomicBool>,
    pub user_data_ws: RwLock<Option<UserDataStreamClient>>,
}

impl BinanceWsState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            ws: RwLock::new(None),
            user_data_running: Arc::new(AtomicBool::new(false)),
            user_data_ws: RwLock::new(None),
        }
    }
}

impl Default for BinanceWsState {
    fn default() -> Self {
        Self::new()
    }
}
