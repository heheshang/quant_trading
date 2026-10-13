//! 初始化阶段 - 各个初始化阶段的实现
//!
//! 将初始化逻辑拆分为独立的阶段，便于理解和测试。

use chrono::Utc;
use data_layer::market_data::DataSource;
use data_layer::market_data_repo::MarketDataRepository;
use data_layer::BinanceDataSource;
use exchange_binance::types::BinanceEnvironment;
use exchange_binance::{Client as BinanceClient, ClientInterface as BinanceClientInterface};
use monitor_layer::LogBuffer;
use quant_clients::RedisCache;
use quant_common::config::AppConfig;
use quant_common::MarketDataProvider;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use trading_layer::BinanceExecutor;

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

// ── 数据库阶段 ──────────────────────────────────────────────────────────────

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

    let repo_pg_client = pg_client
        .as_ref()
        .map(|pg| data_layer::PostgresClient::from_pool(pg.pool().clone()));
    let repo_pg = repo_pg_client.map(Arc::new);

    let alert_repo = repo_pg.as_ref().map(|pg| {
        Arc::new(data_layer::PgAlertRepository::new(Arc::new(
            pg.pool().clone(),
        ))) as Arc<dyn data_layer::AlertRepository>
    });

    let market_data_repo = pg_client
        .as_ref()
        .map(|pg| Arc::new(MarketDataRepository::new(pg.pool().clone())));

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

fn spawn_migration_task(client: Arc<data_layer::PostgresClient>, log_buffer: Arc<LogBuffer>) {
    tokio::spawn(async move {
        run_migrations_with_retry(client, log_buffer).await;
    });
}

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

// ── Redis 阶段 ─────────────────────────────────────────────────────────────

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

// ── Binance 阶段 ────────────────────────────────────────────────────────────

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
