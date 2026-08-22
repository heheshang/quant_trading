use data_layer::{OkxDataSource, PostgresClient};
use exchange_binance::{websocket::BinanceWebSocket, ClientInterface as BinanceClientInterface};
use exchange_okx::websocket::OkxWebSocket;
use exchange_okx::ClientInterface;
use monitor_layer::{AlertManager, LogBuffer};
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_services::AppServices;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_layer::{OkxExecutor, OrderManager};

type SharedClient = Arc<RwLock<dyn ClientInterface + Send + Sync>>;

/// WebSocket 连接状态
pub struct WsState {
    pub running: Arc<AtomicBool>,
    pub ws: RwLock<Option<OkxWebSocket>>,
}

impl WsState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            ws: RwLock::new(None),
        }
    }
}

/// Binance WebSocket 连接状态
pub struct BinanceWsState {
    pub running: Arc<AtomicBool>,
    pub ws: RwLock<Option<BinanceWebSocket>>,
}

impl BinanceWsState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            ws: RwLock::new(None),
        }
    }
}

#[expect(dead_code)]
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub alert_manager: Arc<AlertManager>,
    pub log_buffer: Arc<LogBuffer>,
    pub pg_client: Option<Arc<PostgresClient>>,
    pub redis_cache: Option<RedisCache>,
    pub okx_client: Arc<RwLock<Option<SharedClient>>>,
    pub okx_executor: Arc<RwLock<Option<Arc<OkxExecutor>>>>,
    pub okx_data_source: Arc<RwLock<Option<OkxDataSource>>>,
    pub binance_client: Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>,
    pub order_manager: OrderManager,
    pub app_services: Option<AppServices>,
    pub ws_state: WsState,
    pub binance_ws_state: BinanceWsState,
}
