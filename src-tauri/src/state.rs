use data_layer::PostgresClient;
use exchange_binance::{websocket::BinanceWebSocket, ClientInterface as BinanceClientInterface};
use monitor_layer::{AlertManager, LogBuffer};
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_services::AppServices;
use security::AuditLogger;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_layer::OrderManager;

/// 已认证用户（服务端 RBAC 会话主体）。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthedUser {
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

#[allow(dead_code)]
impl AuthedUser {
    /// 构造一个 admin 测试会话（供测试 seed 使用）。
    pub fn admin() -> Self {
        Self {
            user_id: 1,
            username: "admin".to_string(),
            role: "admin".to_string(),
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
    pub audit_logger: Arc<AuditLogger>,
    pub pg_client: Option<Arc<PostgresClient>>,
    pub redis_cache: Option<RedisCache>,
    pub binance_client: Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>,
    pub order_manager: OrderManager,
    pub app_services: Option<AppServices>,
    pub binance_ws_state: BinanceWsState,
    /// 当前已认证会话；`None` 表示未登录。
    pub auth_session: Arc<RwLock<Option<AuthedUser>>>,
}

impl AppState {
    /// 要求已登录，返回当前会话主体；未登录则返回 `Err`。
    pub async fn require_auth(&self) -> Result<AuthedUser, String> {
        self.auth_session
            .read()
            .await
            .clone()
            .ok_or_else(|| "Authentication required: not logged in".to_string())
    }

    /// 要求当前会话角色达到 `min_role`（`admin` 可越过一切角色）。
    pub async fn require_role(&self, min_role: &str) -> Result<AuthedUser, String> {
        let user = self.require_auth().await?;
        if user.role == "admin" || user.role == min_role {
            Ok(user)
        } else {
            Err(format!("Permission denied: required role {}", min_role))
        }
    }
}
