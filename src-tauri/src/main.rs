// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use quant_common::config::AppConfig;
use quant_common::MarketDataProvider;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

mod commands;
mod prometheus;
mod state;

use data_layer::market_data::DataSource;
use data_layer::market_data_repo::MarketDataRepository;
use data_layer::BinanceDataSource;
use exchange_binance::types::BinanceEnvironment;
use exchange_binance::{Client as BinanceClient, ClientInterface as BinanceClientInterface};
use monitor_layer::{AlertManager, LogBuffer, ACCOUNT_BALANCE, DAILY_PNL, POSITION_VALUE};
use tauri::Manager;
use quant_clients::RedisCache;
use quant_services::AppServices;
use state::AppState;
use trading_layer::{BinanceExecutor, OrderManager};

/// Resolve an Ed25519 private-key path relative to the process cwd, falling
/// back to parent directories. The Tauri dev process often runs from
/// `src-tauri/` while `.env` and the key live at the project root.
fn read_private_key(path: Option<&str>) -> Option<String> {
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

#[tokio::main]
async fn main() {
    // 加载 .env 环境变量
    dotenv::dotenv().ok();

    // 加载配置；容器部署通过 DATABASE_*/REDIS_* 等环境变量覆盖默认值
    let config = AppConfig::from_env();
    // 安全密钥必须替换默认占位值，否则拒绝启动（防伪造 JWT / 解密密钥）。
    if let Err(e) = config.validate_secrets() {
        panic!("安全配置校验失败，拒绝启动：{}", e);
    }

    // 初始化日志（从 AppConfig 读取，LOG_LEVEL 环境变量可覆写）
    let log_level =
        std::env::var("LOG_LEVEL").unwrap_or_else(|_| config.monitoring.log_level.clone());
    monitor_layer::logging::init_logging(monitor_layer::logging::LoggingConfig {
        log_level,
        log_dir: config.monitoring.log_dir.clone(),
        service_name: config.monitoring.service_name.clone(),
        enable_json_logging: config.monitoring.enable_json_logging,
        enable_file_logging: config.monitoring.enable_file_logging,
        enable_stdout_logging: config.monitoring.enable_stdout_logging,
    })
    .expect("Failed to initialize logging");

    info!("Starting Quant Trading System...");

    // 初始化应用状态（以便日志记录可用）
    let log_buffer = Arc::new(LogBuffer::new(1000));

    // 添加初始日志条目
    log_buffer
        .add_entry(quant_common::types::LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: "Quant Trading System initialized successfully".to_string(),
            module: Some("main".to_string()),
        })
        .await;

    // Binance is the live exchange for real order execution and market data.

    // 初始化数据库连接池（懒加载，不阻塞 Tauri 启动）。
    let pg_client = match data_layer::PostgresClient::new_lazy(&config.database) {
        Ok(client) => {
            info!("PostgreSQL connection pool configured; migrations will run in background");
            Some(Arc::new(client))
        }
        Err(e) => {
            warn!("Failed to configure PostgreSQL connection pool: {}", e);
            None
        }
    };

    if let Some(ref client) = pg_client {
        let migration_client = client.clone();
        let migration_log = log_buffer.clone();
        tokio::spawn(async move {
            // 上限重试，避免无限后台循环；达到上限后升级为 error 并停止（不静默 fail-open）。
            let max_attempts = 60;
            for attempt in 1..=max_attempts {
                match migration_client.run_migrations().await {
                    Ok(()) => {
                        info!("Database migrations completed successfully");
                        break;
                    }
                    Err(e) => {
                        if attempt >= max_attempts {
                            tracing::error!(
                                "Database migrations failed after {max_attempts} attempts; services may be degraded: {}",
                                e
                            );
                            migration_log
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
                        migration_log
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
        });
    } else {
        warn!("PostgreSQL connection pool unavailable, running without database");
    }

    // 为 AppServices 创建 data_layer::PostgresClient，复用同一个 PgPool，
    // 避免为同一数据库打开第二套连接池。
    let repo_pg_client = pg_client
        .as_ref()
        .map(|pg| data_layer::PostgresClient::from_pool(pg.pool().clone()));
    let repo_pg = repo_pg_client.map(Arc::new);
    // 告警持久化仓储（有 DB 时注入 AlertManager，无 DB 时降级内存）
    let alert_repo = repo_pg.as_ref().map(|pg| {
        Arc::new(data_layer::PgAlertRepository::new(Arc::new(
            pg.pool().clone(),
        ))) as Arc<dyn data_layer::AlertRepository>
    });
    let alert_manager = Arc::new(AlertManager::new(false, vec![]).with_repository(alert_repo));

    // 行情快照仓储（供市场数据读接口使用）
    let market_data_repo = pg_client
        .as_ref()
        .map(|pg| Arc::new(MarketDataRepository::new(pg.pool().clone())));
    // 审计日志仓储（有 DB 时持久化，无 DB 时仅内存日志）
    let audit_repo = repo_pg.as_ref().map(|pg| {
        Arc::new(data_layer::PgAuditRepository::new(Arc::new(
            pg.pool().clone(),
        ))) as Arc<dyn data_layer::AuditRepository>
    });
    let audit_logger = Arc::new(security::AuditLogger::new(audit_repo));

    let redis_cache = RedisCache::new(&config.redis).ok();
    if redis_cache.is_some() {
        info!("Redis connection established successfully");
    } else {
        warn!("Redis connection failed, running without cache");
    }

    // Create shared OrderManager
    let order_manager = OrderManager::new();

    // Binance client + executor (optional). Built before `config` is moved into
    // `config_arc` so we can read `config.binance`. The service client feeds
    // `BinanceService` (market data); the executor feeds `OrderProcessor` so
    // real order execution routes through the Binance live exchange. The same
    // client backs `BinanceDataSource` (the market-data data source supplied to
    // `MarketService` and the persistence-first live fallback).
    let binance_client_shared: Arc<RwLock<Option<Arc<dyn BinanceClientInterface + Send + Sync>>>>;
    let binance_executor_shared: Arc<RwLock<Option<Arc<BinanceExecutor>>>>;
    let market_data_source: Arc<RwLock<Option<Arc<dyn DataSource>>>>;
    let live_market_data_provider: Option<Arc<dyn MarketDataProvider>>;
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

        // Build the signed client; validate once and reuse the same params for
        // the market-data and execution clients (identical args → same result).
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
            Ok(client) => {
                let service_client: Arc<dyn BinanceClientInterface + Send + Sync> =
                    Arc::new(client);
                let market_data_client: Arc<RwLock<dyn BinanceClientInterface + Send + Sync>> =
                    Arc::new(RwLock::new(build_client().expect("Binance client built")));
                binance_client_shared = Arc::new(RwLock::new(Some(service_client)));

                let binance_ds = Arc::new(BinanceDataSource::new(market_data_client));
                market_data_source =
                    Arc::new(RwLock::new(Some(binance_ds.clone() as Arc<dyn DataSource>)));
                live_market_data_provider =
                    Some(binance_ds.clone() as Arc<dyn MarketDataProvider>);

                let executor_client = Arc::new(RwLock::new(
                    build_client().expect("Binance client built"),
                ));
                binance_executor_shared = Arc::new(RwLock::new(Some(Arc::new(
                    BinanceExecutor::new(executor_client),
                ))));
            }
            Err(e) => {
                error!("Binance client initialization failed: {e}");
                info!("Binance integration disabled");
                binance_client_shared = Arc::new(RwLock::new(None));
                binance_executor_shared = Arc::new(RwLock::new(None));
                market_data_source = Arc::new(RwLock::new(None));
                live_market_data_provider = None;
            }
        }
    } else {
        info!("Binance integration disabled");
        binance_client_shared = Arc::new(RwLock::new(None));
        binance_executor_shared = Arc::new(RwLock::new(None));
        market_data_source = Arc::new(RwLock::new(None));
        live_market_data_provider = None;
    }

    let config_arc = Arc::new(RwLock::new(config));

    // Create AppServices. Config is loaded from env via dotenv at startup;
    // the previous `config.toml` file persistence is replaced by dotenv.
    // Infrastructure is grouped into a single `SharedInfra` bundle (DIP/SRP).
    let infra = quant_services::SharedInfra {
        config: config_arc.clone(),
        postgres: repo_pg.clone(),
        redis: None,
        market_data: market_data_repo.clone(),
        market_data_source,
        live_market_data_provider,
        binance_client: binance_client_shared.clone(),
        binance_executor: binance_executor_shared.clone(),
        order_manager: Arc::new(order_manager.clone()),
        log_buffer: log_buffer.clone(),
    };
    let app_services = AppServices::new(infra);
    // 启动纸面订单执行调度器（后台定时任务）：按模拟延迟撮合已到期纸面单，
    // 以 DB 为事实来源，重启后可恢复。
    app_services
        .order_processor
        .start_paper_execution_scheduler()
        .await;
    // Capture the Prometheus monitoring config before `config_arc` is moved
    // into `AppState` below.
    let prometheus_cfg = config_arc.read().await.monitoring.clone();

    let app_state = AppState {
        config: config_arc,
        alert_manager,
        log_buffer,
        audit_logger,
        pg_client,
        redis_cache,
        binance_client: binance_client_shared,
        order_manager,
        app_services: Some(app_services),
        binance_ws_state: state::BinanceWsState::new(),
        auth_session: Arc::new(RwLock::new(None)),
    };

    // 初始化指标收集
    monitor_layer::MetricsCollector::init();

    // Metrics start at 0 — actual values are set by the data poller / account service
    ACCOUNT_BALANCE.set(0.0);
    POSITION_VALUE.set(0.0);
    DAILY_PNL.set(0.0);

    info!("Application initialized successfully");
    // Start the Prometheus /metrics endpoint (if enabled)
    {
        let monitoring = prometheus_cfg;
        if monitoring.enable_prometheus {
            let port = monitoring.prometheus_port;
            info!("Starting Prometheus metrics server on port {}", port);
            tokio::spawn(async move {
                prometheus::run(port).await;
            });
        } else {
            info!("Prometheus metrics endpoint disabled");
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(app_state)
        .setup(|app| {
            // 启动实盘订单状态监控（后台轮询 Binance + 同步 live_trades + 推事件）。
            let state = app.state::<state::AppState>();
            if let Some(services) = state.app_services.as_ref() {
                commands::binance_ws::start_live_order_monitor(app.handle().clone(), services);
                // 资产曲线后台快照写入器（独立于前端刷新，每 60s 记录权益）。
                commands::binance_ws::start_equity_snapshot_writer(services);
                // 指标 Gauge 每 5s 刷新真实账户值（Monitor 指标监控）。
                commands::binance_ws::start_monitor_metrics(services);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::update_config,
            commands::secure_encrypt,
            commands::secure_decrypt,
            commands::get_market_data,
            commands::submit_order,
            commands::run_algorithmic_order,
            commands::get_account_info,
            commands::get_positions,
            commands::get_active_orders,
            commands::get_recent_orders,
            commands::cancel_order,
            commands::run_backtest,
            commands::get_backtest_results,
            commands::get_backtest_result,
            commands::delete_backtest_result,
            commands::get_metrics,
            commands::get_alerts,
            commands::acknowledge_alert,
            commands::get_logs,
            commands::check_redis_status,
            commands::get_audit_logs,
            commands::save_api_key,
            commands::get_api_keys,
            commands::get_funding_rates,
            commands::get_mark_prices,
            commands::get_symbols,
            commands::get_klines,
            commands::get_trades,
            commands::get_orderbook,
            commands::get_balances,
            commands::get_last_prices,
            commands::get_ticker_snapshots,
            commands::get_account_snapshots,
            commands::record_account_snapshot,
            commands::get_position_snapshots,
            commands::get_strategies,
            commands::save_strategy,
            commands::delete_strategy,
            commands::toggle_strategy,
            commands::deploy_strategy,
            commands::start_strategy,
            commands::stop_strategy,
            commands::pause_strategy,
            commands::resume_strategy,
            commands::archive_strategy,
            commands::list_strategy_types,
            commands::get_strategy_type_info,
            commands::create_strategy,
            commands::get_risk_metrics,
            commands::get_risk_config,
            commands::update_risk_config,
            commands::pre_trade_check,
            commands::login,
            commands::verify_token,
            commands::update_profile,
            commands::change_password,
            commands::get_user_profile,
            commands::enable_2fa,
            commands::verify_2fa_code,
            commands::disable_2fa,
            commands::optimize_strategy,
            // Binance commands
            commands::get_binance_balance,
            commands::get_binance_candles,
            commands::get_binance_ticker_prices,
            commands::get_live_trades,
            commands::get_binance_order_book,
            commands::get_binance_positions,
            commands::get_binance_orders,
            commands::get_binance_order,
            commands::get_binance_instruments,
            commands::place_binance_order,
            commands::cancel_binance_order,
            commands::check_binance_status,
            // Binance WebSocket commands
            commands::start_binance_market_data,
            commands::subscribe_binance_candle,
            commands::subscribe_binance_depth,
            commands::subscribe_binance_ticker,
            commands::subscribe_binance_trades,
            commands::subscribe_binance_orderbook,
            commands::stop_binance_market_data,
            commands::get_binance_subscriptions,
            commands::start_binance_user_data_stream,
            commands::stop_binance_user_data_stream,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
