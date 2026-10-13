//! 初始化模块 - 应用启动时的核心初始化逻辑
//!
//! 职责：
//! - 配置文件加载与验证
//! - 数据库连接池初始化
//! - Redis 缓存初始化
//! - Binance 客户端初始化
//! - AppState 构建

use crate::state::{AppState, BinanceWsState};
use chrono::Utc;
use data_layer::market_data::DataSource;
use data_layer::market_data_repo::MarketDataRepository;
use data_layer::BinanceDataSource;
use exchange_binance::types::BinanceEnvironment;
use exchange_binance::{Client as BinanceClient, ClientInterface as BinanceClientInterface};
use monitor_layer::{AlertManager, LogBuffer};
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_common::MarketDataProvider;
use quant_services::{AppServices, SharedInfra};
use security::AuditLogger;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use trading_layer::{BinanceExecutor, OrderManager};

/// 数据库配置结果
pub struct DatabaseConfig {
    /// PostgreSQL 客户端
    pub pg_client: Option<Arc<data_layer::PostgresClient>>,
    /// 行情数据仓储
    pub market_data_repo: Option<Arc<MarketDataRepository>>,
    /// 告警仓储
    pub alert_repo: Option<Arc<dyn data_layer::AlertRepository>>,
    /// 审计日志仓储
    pub audit_repo: Option<Arc<dyn data_layer::AuditRepository>>,
}

/// Binance 配置结果
pub struct BinanceConfig {
    /// Binance API 客户端
    pub client: Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>,
    /// Binance 执行器
    pub executor: Arc<RwLock<Option<Arc<BinanceExecutor>>>>,
    /// 市场数据源
    pub market_data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>,
    /// 实时行情提供者
    pub market_data_provider: Option<Arc<dyn MarketDataProvider>>,
}

/// 解析 Ed25519 私钥路径
///
/// 优先使用绝对路径，fallback 到相对路径向上查找（支持 Tauri dev 模式）
pub fn read_private_key(path: Option<&str>) -> Option<String> {
    let path = path?;
    let as_path = std::path::Path::new(path);
    if as_path.is_absolute() {
        return std::fs::read_to_string(as_path).ok();
    }
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        if let Ok(txt) = std::fs::read_to_string(dir.join(as_path)) {
            return Some(txt);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// 初始化数据库连接池
///
/// 返回懒加载的客户端，迁移在后台异步执行
pub async fn init_database(config: &AppConfig, log_buffer: &Arc<LogBuffer>) -> DatabaseConfig {
    let pg_client = match data_layer::PostgresClient::new_lazy(&config.database) {
        Ok(client) => {
            info!("PostgreSQL connection pool configured; migrations will run in background");
            let client = Arc::new(client);
            spawn_migration_task(client.clone(), log_buffer.clone());
            Some(client)
        }
        Err(e) => {
            warn!("Failed to configure PostgreSQL connection pool: {}", e);
            None
        }
    };

    // 创建复用 PgPool 的客户端
    let repo_pg_client = pg_client
        .as_ref()
        .map(|pg| data_layer::PostgresClient::from_pool(pg.pool().clone()));
    let repo_pg = repo_pg_client.map(Arc::new);

    // 告警持久化仓储
    let alert_repo = repo_pg.as_ref().map(|pg| {
        Arc::new(data_layer::PgAlertRepository::new(Arc::new(
            pg.pool().clone(),
        ))) as Arc<dyn data_layer::AlertRepository>
    });

    // 行情快照仓储
    let market_data_repo = pg_client
        .as_ref()
        .map(|pg| Arc::new(MarketDataRepository::new(pg.pool().clone())));

    // 审计日志仓储
    let audit_repo = repo_pg.as_ref().map(|pg| {
        Arc::new(data_layer::PgAuditRepository::new(Arc::new(
            pg.pool().clone(),
        ))) as Arc<dyn data_layer::AuditRepository>
    });

    DatabaseConfig {
        pg_client,
        market_data_repo,
        alert_repo,
        audit_repo,
    }
}

/// 启动数据库迁移任务（后台运行，带重试）
fn spawn_migration_task(client: Arc<data_layer::PostgresClient>, log_buffer: Arc<LogBuffer>) {
    tokio::spawn(async move {
        run_migrations_with_retry(client, log_buffer).await;
    });
}

/// 执行迁移，带最多 60 次重试
async fn run_migrations_with_retry(
    client: Arc<data_layer::PostgresClient>,
    log_buffer: Arc<LogBuffer>,
) {
    let max_attempts = 60;
    for attempt in 1..=max_attempts {
        match client.run_migrations().await {
            Ok(()) => {
                info!("Database migrations completed successfully");
                break;
            }
            Err(e) => {
                if attempt >= max_attempts {
                    error!(
                        "Database migrations failed after {max_attempts} attempts; services may be degraded: {}",
                        e
                    );
                    log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "error".to_string(),
                            message: format!(
                                "数据库迁移连续失败({max_attempts}次)，服务可能降级：{}",
                                e
                            ),
                            module: Some("database".to_string()),
                        })
                        .await;
                    break;
                }
                warn!(
                    "Database connection/migration failed, will retry in 5s: {}",
                    e
                );
                log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: Utc::now(),
                        level: "warn".to_string(),
                        message: format!("Database unavailable, retrying in 5s: {}", e),
                        module: Some("database".to_string()),
                    })
                    .await;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// 初始化 Redis 缓存
pub fn init_redis(config: &AppConfig) -> Option<RedisCache> {
    match RedisCache::new(&config.redis) {
        Ok(cache) => {
            info!("Redis connection established successfully");
            Some(cache)
        }
        Err(e) => {
            warn!("Redis connection failed, running without cache: {}", e);
            None
        }
    }
}

/// 初始化 Binance 客户端和执行器
pub fn init_binance(config: &AppConfig) -> BinanceConfig {
    let client: Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>;
    let executor: Arc<RwLock<Option<Arc<BinanceExecutor>>>>;
    let market_data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>;
    let market_data_provider: Option<Arc<dyn MarketDataProvider>>;

    if config.binance.enable && !config.binance.api_key.is_empty() {
        info!(
            "Binance client initialized in {} mode",
            config.binance.environment
        );
        let environment = BinanceEnvironment::parse(&config.binance.environment);
        let private_key_pem = read_private_key(config.binance.private_key_path.as_deref());

        info!(
            "Binance config: env={} key_type={} base_url={} ws_url={} private_key_loaded={}{}",
            config.binance.environment,
            config.binance.key_type,
            config.binance.base_url.as_deref().unwrap_or("<unset>"),
            config.binance.ws_url.as_deref().unwrap_or("<unset>"),
            private_key_pem.is_some(),
            private_key_pem
                .as_deref()
                .map(|p| format!(" ({} bytes)", p.len()))
                .unwrap_or_default(),
        );

        let build_client = || {
            BinanceClient::new(
                config.binance.api_key.clone(),
                config.binance.api_secret.clone(),
                environment,
                config.binance.base_url.clone(),
                config.binance.key_type.clone(),
                private_key_pem.clone(),
            )
        };

        match build_client() {
            Ok(binance_client) => {
                let service_client: Arc<dyn BinanceClientInterface + Send + Sync> =
                    Arc::new(binance_client);
                let market_data_client: Arc<RwLock<dyn BinanceClientInterface + Send + Sync>> =
                    Arc::new(RwLock::new(build_client().expect("Binance client built")));
                client = Arc::new(RwLock::new(Some(service_client)));

                let binance_ds = Arc::new(BinanceDataSource::new(market_data_client));
                market_data_source =
                    Arc::new(RwLock::new(Some(binance_ds.clone() as Arc<dyn DataSource>)));
                market_data_provider = Some(binance_ds.clone() as Arc<dyn MarketDataProvider>);

                let executor_client =
                    Arc::new(RwLock::new(build_client().expect("Binance client built")));
                executor = Arc::new(RwLock::new(Some(Arc::new(BinanceExecutor::new(
                    executor_client,
                )))));
            }
            Err(e) => {
                error!("Binance client initialization failed: {e}");
                client = Arc::new(RwLock::new(None));
                executor = Arc::new(RwLock::new(None));
                market_data_source = Arc::new(RwLock::new(None));
                market_data_provider = None;
            }
        }
    } else {
        info!("Binance integration disabled");
        client = Arc::new(RwLock::new(None));
        executor = Arc::new(RwLock::new(None));
        market_data_source = Arc::new(RwLock::new(None));
        market_data_provider = None;
    }

    BinanceConfig {
        client,
        executor,
        market_data_source,
        market_data_provider,
    }
}

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

    // 在移动 config 之前读取 monitoring 配置
    let monitoring_config = config
        .try_read()
        .map(|c| c.monitoring.clone())
        .unwrap_or_else(|_| {
            // 提供默认值
            quant_common::config::MonitoringConfig {
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
            }
        });

    // SharedInfra 中 Redis 设为 None（原始代码行为）
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
