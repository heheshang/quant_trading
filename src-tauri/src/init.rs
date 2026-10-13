//! 初始化模块 - 应用启动时的核心初始化逻辑
//!
//! 职责：
//! - 配置文件加载与验证
//! - 数据库连接池初始化
//! - Redis 缓存初始化
//! - Binance 客户端初始化
//! - AppState 构建
//!
//! # 模块结构
//! - `phases.rs`: 各初始化阶段的实现

pub mod phases;

// 重新导出 phases 中的类型
pub use phases::{init_binance, init_database, init_redis, BinanceConfig, DatabaseConfig};

use crate::state::{AppState, BinanceWsState};
use monitor_layer::{AlertManager, LogBuffer};
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_services::{AppServices, SharedInfra};
use security::AuditLogger;
use std::sync::Arc;
use tokio::sync::RwLock;
use trading_layer::OrderManager;

/// 构建完整的 AppState 和 AppServices
pub fn build_app_state(
    config: Arc<RwLock<AppConfig>>,
    db: DatabaseConfig,
    binance: BinanceConfig,
    redis: Option<RedisCache>,
    log_buffer: Arc<LogBuffer>,
    order_manager: OrderManager,
) -> (AppState, quant_common::config::MonitoringConfig) {
    let alert_manager = Arc::new(AlertManager::new(false, vec![]).with_repository(db.alert_repo));
    let audit_logger = Arc::new(AuditLogger::new(db.audit_repo));

    let monitoring_config = config
        .try_read()
        .map(|c| c.monitoring.clone())
        .unwrap_or_else(|_e| quant_common::config::MonitoringConfig {
            enable_prometheus: false,
            prometheus_port: 9090,
            log_level: "info".to_string(),
            log_dir: "logs".to_string(),
            service_name: "quant-trading".to_string(),
            enable_json_logging: false,
            enable_file_logging: true,
            enable_stdout_logging: true,
            alert_email: None,
            alert_webhook: None,
        });

    let infra = SharedInfra {
        config: config.clone(),
        postgres: db.pg_client.clone(),
        redis: None,
        market_data: db.market_data_repo.clone(),
        market_data_source: binance.market_data_source.clone(),
        live_market_data_provider: binance.market_data_provider.clone(),
        binance_client: binance.client.clone(),
        binance_executor: binance.executor.clone(),
        order_manager: Arc::new(order_manager.clone()),
        log_buffer: log_buffer.clone(),
    };

    let app_services = AppServices::new(infra);

    let app_state = AppState {
        config,
        alert_manager,
        log_buffer,
        audit_logger,
        pg_client: db.pg_client,
        redis_cache: redis,
        binance_client: binance.client,
        order_manager,
        app_services: Some(app_services),
        binance_ws_state: BinanceWsState::new(),
        auth_session: Arc::new(RwLock::new(None)),
    };

    (app_state, monitoring_config)
}
