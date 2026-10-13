//! 状态管理模块
//!
//! 包含 `AppState` 和相关状态类型。

pub mod runtime;

use data_layer::PostgresClient;
use exchange_binance::ClientInterface as BinanceClientInterface;
use monitor_layer::{AlertManager, LogBuffer};
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_services::AppServices;
use security::AuditLogger;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_layer::OrderManager;

// 重新导出子模块的类型
pub use runtime::BinanceWsState;

/// 已认证用户（服务端 RBAC 会话主体）。
#[derive(Debug, Clone)]
pub struct AuthedUser {
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

impl AuthedUser {
    /// 构造一个 admin 测试会话（供测试 seed 使用）。
    #[cfg(test)]
    pub fn admin() -> Self {
        Self {
            user_id: 1,
            username: "admin".to_string(),
            role: "admin".to_string(),
        }
    }
}

/// 统一的应用状态
///
/// 包含配置、服务、运行时等所有共享状态。
/// 状态按职责分组，便于理解和维护。
#[expect(dead_code)]
pub struct AppState {
    // ── 配置状态 ──────────────────────────────────────────────────────────
    /// 应用配置
    pub config: Arc<RwLock<AppConfig>>,

    // ── 基础设施状态 ──────────────────────────────────────────────────────
    /// PostgreSQL 客户端
    pub pg_client: Option<Arc<PostgresClient>>,
    /// Redis 缓存
    pub redis_cache: Option<RedisCache>,
    /// Binance API 客户端
    pub binance_client: Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>,
    /// 订单管理器
    pub order_manager: OrderManager,

    // ── 监控和日志 ───────────────────────────────────────────────────────
    /// 告警管理器
    pub alert_manager: Arc<AlertManager>,
    /// 日志缓冲区
    pub log_buffer: Arc<LogBuffer>,
    /// 审计日志记录器
    pub audit_logger: Arc<AuditLogger>,

    // ── 业务服务 ──────────────────────────────────────────────────────────
    /// 业务服务容器
    pub app_services: Option<AppServices>,

    // ── 运行时状态 ───────────────────────────────────────────────────────
    /// WebSocket 连接状态
    pub binance_ws_state: BinanceWsState,

    // ── 会话状态 ──────────────────────────────────────────────────────────
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
