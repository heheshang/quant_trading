// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use quant_common::config::AppConfig;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

mod commands;
mod state;

use state::AppState;
    use monitor_layer::{ORDERS_TOTAL, ORDERS_FILLED, ORDERS_CANCELLED, ACCOUNT_BALANCE, POSITION_VALUE, DAILY_PNL};

#[tokio::main]
async fn main() {
    // 初始化日志
    monitor_layer::logging::init_logging(monitor_layer::logging::LoggingConfig {
        log_level: "info".to_string(),
        log_dir: "./logs".to_string(),
    });

    info!("Starting Quant Trading System...");

    // 加载配置
    let config = AppConfig::default();

    // 初始化数据库连接（可选，需要数据库运行）
    // let pg_client = PostgresClient::new(&config.database).await.ok();
    // let redis_cache = RedisCache::new(&config.redis).ok();
    // let timeseries_db = TimeSeriesDB::new(&config.influxdb).ok();

    // 初始化应用状态
    let app_state = AppState {
        config: Arc::new(RwLock::new(config)),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
