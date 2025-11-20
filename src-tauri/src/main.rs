// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use quant_common::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use chrono::Utc;

mod commands;
mod state;

use state::AppState;
use monitor_layer::{ACCOUNT_BALANCE, POSITION_VALUE, DAILY_PNL, AlertManager, LogBuffer};
use exchange_okx::{Client as OkxClient, types::OkxEnvironment};
use trading_layer::OkxExecutor;
use data_layer::OkxDataSource;

#[tokio::main]
async fn main() {
    // 初始化日志
    monitor_layer::logging::init_logging(monitor_layer::logging::LoggingConfig {
        log_level: "info".to_string(),
        log_dir: "./logs".to_string(),
        service_name: "quant-trading".to_string(),
        enable_json_logging: false,
        enable_file_logging: true,
        enable_stdout_logging: true,
    }).expect("Failed to initialize logging");

    info!("Starting Quant Trading System...");

    // 加载配置
    let config = AppConfig::default();
    
    // 初始化 OKX 客户端
    let (okx_client, okx_executor, okx_data_source) = if config.okx.enable && !config.okx.api_key.is_empty() {
        match OkxClient::new(
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
                info!("OKX client initialized successfully in {} mode", config.okx.environment);
                
                // Wrap client in Arc for sharing across executor and data source
                let client_arc = Arc::new(RwLock::new(client));
                
                // Create executor and data source from the Arc'd client
                let executor = OkxExecutor::new(client_arc.clone());
                let data_source = OkxDataSource::new(client_arc.clone());
                
                // Keep a reference to the client for direct API calls
                (Some(client_arc), Some(executor), Some(data_source))
            }
            Err(e) => {
                warn!("Failed to initialize OKX client: {}", e);
                (None, None, None)
            }
        }
    } else {
        info!("OKX integration disabled");
        (None, None, None)
    };

    // 初始化数据库连接（可选，需要数据库运行）
    // let pg_client = PostgresClient::new(&config.database).await.ok();
    // let redis_cache = RedisCache::new(&config.redis).ok();
    // let timeseries_db = TimeSeriesDB::new(&config.influxdb).ok();

    // 初始化应用状态
    let alert_manager = Arc::new(AlertManager::new(false, vec![]));
    let log_buffer = Arc::new(LogBuffer::new(1000));
    
    // 添加初始日志条目
    log_buffer.add_entry(quant_common::types::LogEntry {
        timestamp: Utc::now(),
        level: "info".to_string(),
        message: "Quant Trading System initialized successfully".to_string(),
        module: Some("main".to_string()),
    }).await;
    
    let app_state = AppState {
        config: Arc::new(RwLock::new(config)),
        alert_manager,
        log_buffer,
        okx_client: Arc::new(RwLock::new(okx_client)),
        okx_executor: Arc::new(RwLock::new(okx_executor)),
        okx_data_source: Arc::new(RwLock::new(okx_data_source)),
    };

    // 初始化指标收集
    monitor_layer::MetricsCollector::init();
    
    // 初始化一些默认指标值
    ACCOUNT_BALANCE.set(1234567.89);
    POSITION_VALUE.set(1000000.0);
    DAILY_PNL.set(12345.67);

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
            commands::get_metrics,
            commands::get_alerts,
            commands::acknowledge_alert,
            commands::get_logs,
            commands::get_strategies,
            commands::save_strategy,
            commands::delete_strategy,
            commands::toggle_strategy,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
