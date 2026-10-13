// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 模块化拆分：
// - init.rs: 应用初始化（配置、数据库、Redis、Binance）
// - background.rs: 后台任务管理
// - commands/: Tauri 命令处理器

mod background;
mod commands;
mod init;
mod prometheus;
mod state;

use init::{init_binance, init_database, init_redis};
use quant_common::config::AppConfig;
use std::sync::Arc;
use tauri::Manager;
use tracing::info;

#[tokio::main]
async fn main() {
    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 1: 环境变量和配置加载
    // ─────────────────────────────────────────────────────────────────────────
    dotenv::dotenv().ok();
    let config = AppConfig::from_env();

    // 安全密钥必须替换默认占位值，否则拒绝启动（防伪造 JWT / 解密密钥）
    if let Err(e) = config.validate_secrets() {
        panic!("安全配置校验失败，拒绝启动：{}", e);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 2: 日志系统初始化
    // ─────────────────────────────────────────────────────────────────────────
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

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 3: 日志缓冲区初始化
    // ─────────────────────────────────────────────────────────────────────────
    let log_buffer = Arc::new(monitor_layer::LogBuffer::new(1000));
    log_buffer
        .add_entry(quant_common::types::LogEntry {
            timestamp: chrono::Utc::now(),
            level: "info".to_string(),
            message: "Quant Trading System initialized successfully".to_string(),
            module: Some("main".to_string()),
        })
        .await;

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 4: 数据库初始化
    // ─────────────────────────────────────────────────────────────────────────
    let db_config = init_database(&config, &log_buffer).await;

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 5: Redis 缓存初始化
    // ─────────────────────────────────────────────────────────────────────────
    let redis_cache = init_redis(&config);

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 6: OrderManager 初始化
    // ─────────────────────────────────────────────────────────────────────────
    let order_manager = trading_layer::OrderManager::new();

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 7: Binance 客户端初始化
    // ─────────────────────────────────────────────────────────────────────────
    let binance_config = init_binance(&config);

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 8: 构建 AppState 和 AppServices
    // ─────────────────────────────────────────────────────────────────────────
    let (app_state, monitoring_config) = init::build_app_state(
        Arc::new(tokio::sync::RwLock::new(config)),
        db_config,
        binance_config,
        redis_cache,
        log_buffer,
        order_manager,
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 9: 启动纸面订单执行调度器
    // ─────────────────────────────────────────────────────────────────────────
    if let Some(ref services) = app_state.app_services {
        services
            .order_processor
            .start_paper_execution_scheduler()
            .await;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 10: 初始化监控指标
    // ─────────────────────────────────────────────────────────────────────────
    monitor_layer::MetricsCollector::init();
    monitor_layer::ACCOUNT_BALANCE.set(0.0);
    monitor_layer::POSITION_VALUE.set(0.0);
    monitor_layer::DAILY_PNL.set(0.0);

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 11: 启动 Prometheus 服务器
    // ─────────────────────────────────────────────────────────────────────────
    if monitoring_config.enable_prometheus {
        let port = monitoring_config.prometheus_port;
        info!("Starting Prometheus metrics server on port {}", port);
        tokio::spawn(async move {
            prometheus::run(port).await;
        });
    } else {
        info!("Prometheus metrics endpoint disabled");
    }

    info!("Application initialized successfully");

    // ─────────────────────────────────────────────────────────────────────────
    // 阶段 12: 启动 Tauri 应用
    // ─────────────────────────────────────────────────────────────────────────
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(app_state)
        .setup(|app| {
            let state = app.state::<state::AppState>();
            if let Some(services) = state.app_services.as_ref() {
                background::start_background_tasks(app.handle().clone(), services);
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
