// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use quant_common::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

mod commands;
mod state;
mod ws_commands;

use data_layer::{market_data_repo::MarketDataRepository, OkxDataSource};
use data_puller::DataPuller;
use exchange_okx::{types::OkxEnvironment, Client, ClientInterface};
use monitor_layer::{AlertManager, LogBuffer, ACCOUNT_BALANCE, DAILY_PNL, POSITION_VALUE};
use quant_clients::RedisCache;
use quant_services::AppServices;
use state::AppState;
use trading_layer::{OkxExecutor, OrderManager};

#[tokio::main]
async fn main() {
    // 加载 .env 环境变量
    dotenv::dotenv().ok();

    // 加载配置；容器部署通过 DATABASE_*/REDIS_* 等环境变量覆盖默认值
    let config = AppConfig::from_env();

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

    // 初始化应用状态（在 OKX 客户端初始化之前，以便日志记录可用）
    let alert_manager = Arc::new(AlertManager::new(false, vec![]));
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

    // 初始化 OKX 客户端
    let (okx_client, okx_executor, okx_data_source) = if config.okx.enable
        && !config.okx.api_key.is_empty()
    {
        match Client::new(
            config.okx.api_key.clone(),
            config.okx.api_secret.clone(),
            config.okx.passphrase.clone(),
            if config.okx.environment == "live" {
                OkxEnvironment::Live
            } else {
                OkxEnvironment::Demo
            },
        ) {
            Ok(client) => {
                info!(
                    "OKX client initialized successfully in {} mode",
                    config.okx.environment
                );

                // Wrap client in Arc for sharing across executor and data source
                let client_arc = Arc::new(RwLock::new(client));

                // Data source keeps a concrete-client reference.
                let data_source = OkxDataSource::new(client_arc.clone());

                // Coerce to trait object for the shared state (enables mocking in tests).
                // `OkxExecutor` accepts the `ClientInterface` trait object.
                let client_trait: Arc<RwLock<dyn ClientInterface + Send + Sync>> = client_arc;
                // A single shared executor instance for both the command layer and the
                // services container (DIP: one instance, shared via `Arc`).
                let executor = Arc::new(OkxExecutor::new(client_trait.clone()));

                (Some(client_trait), Some(executor), Some(data_source))
            }
            Err(e) => {
                warn!("Failed to initialize OKX client: {}", e);
                log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "error".to_string(),
                            message: format!("OKX client init failed: {}. OKX-dependent features will be unavailable.", e),
                            module: Some("okx".to_string()),
                        })
                        .await;
                (None, None, None)
            }
        }
    } else {
        info!("OKX integration disabled");
        (None, None, None)
    };

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
            loop {
                match migration_client.run_migrations().await {
                    Ok(()) => {
                        info!("Database migrations completed successfully");
                        break;
                    }
                    Err(e) => {
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

    // 为 AppServices 创建 quant_repository::PostgresClient，复用同一个 PgPool，
    // 避免为同一数据库打开第二套连接池。
    let repo_pg_client = pg_client
        .as_ref()
        .map(|pg| quant_repository::PostgresClient::from_pool(pg.pool().clone()));
    let repo_pg = repo_pg_client.map(Arc::new);

    let redis_cache = RedisCache::new(&config.redis).ok();
    if redis_cache.is_some() {
        info!("Redis connection established successfully");
    } else {
        warn!("Redis connection failed, running without cache");
    }

    // 初始化 DataPuller 后台任务
    if let Some(ref client_arc) = okx_client {
        if let Some(ref pg) = pg_client {
            let data_puller = DataPuller::new(
                config.data_puller.clone(),
                client_arc.clone(),
                Arc::new(MarketDataRepository::new(pg.pool().clone())),
            );
            tokio::spawn(async move {
                info!("Starting data puller background task");
                if let Err(e) = data_puller.run().await {
                    tracing::error!("Data puller exited with error: {}", e);
                }
            });
        } else {
            warn!("Data puller skipped: no database connection available");
        }
    } else {
        info!("Data puller skipped: OKX client not initialized");
    }

    // Create shared OrderManager
    let order_manager = OrderManager::new();

    // Create shared Arc wrappers for OKX infrastructure
    let config_arc = Arc::new(RwLock::new(config));
    let okx_client_shared = Arc::new(RwLock::new(okx_client));
    let okx_executor_shared = Arc::new(RwLock::new(okx_executor));
    let okx_data_source_shared = Arc::new(RwLock::new(okx_data_source));

    // Create AppServices with config file path for persistence.
    // Infrastructure is grouped into a single `SharedInfra` bundle (DIP/SRP)
    // instead of a long argument list.
    let config_path = std::path::PathBuf::from("config.toml");
    let infra = quant_services::SharedInfra {
        config: config_arc.clone(),
        postgres: repo_pg.clone(),
        redis: None,
        market_data: None,
        okx_client: okx_client_shared.clone(),
        okx_executor: okx_executor_shared.clone(),
        okx_data_source: okx_data_source_shared.clone(),
        order_manager: Arc::new(order_manager.clone()),
        log_buffer: log_buffer.clone(),
    };
    let app_services = AppServices::with_config_path(infra, config_path);

    let app_state = AppState {
        config: config_arc,
        alert_manager,
        log_buffer,
        pg_client,
        redis_cache,
        okx_client: okx_client_shared,
        okx_executor: okx_executor_shared,
        okx_data_source: okx_data_source_shared,
        order_manager,
        app_services: Some(app_services),
        ws_state: state::WsState::new(),
    };

    // 初始化指标收集
    monitor_layer::MetricsCollector::init();

    // Metrics start at 0 — actual values are set by the data poller / account service
    ACCOUNT_BALANCE.set(0.0);
    POSITION_VALUE.set(0.0);
    DAILY_PNL.set(0.0);

    info!("Application initialized successfully");

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::update_config,
            commands::get_market_data,
            commands::submit_order,
            commands::get_account_info,
            commands::get_positions,
            commands::get_active_orders,
            commands::run_backtest,
            commands::get_backtest_results,
            commands::get_backtest_result,
            commands::delete_backtest_result,
            commands::get_metrics,
            commands::get_alerts,
            commands::acknowledge_alert,
            commands::get_logs,
            commands::check_redis_status,
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
            // OKX commands
            commands::get_okx_balance,
            commands::get_okx_positions,
            commands::place_okx_order,
            commands::cancel_okx_order,
            commands::get_okx_candles,
            commands::get_okx_instruments,
            commands::check_okx_status,
            commands::get_okx_announcements,
            commands::execute_okx_order,
            commands::get_okx_realtime_data,
            commands::get_okx_historical_data,
            // WebSocket commands
            ws_commands::start_market_data,
            ws_commands::subscribe_market_data,
            ws_commands::unsubscribe_market_data,
            ws_commands::stop_market_data,
            ws_commands::get_subscriptions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
