use crate::state::AppState;
use chrono::Utc;
use quant_common::config::AppConfig;
use quant_common::types::{Account, MarketData, Order, Position};
use rust_decimal::prelude::ToPrimitive;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.read().await;
    Ok(config.clone())
}

/// 更新系统配置
#[tauri::command]
pub async fn update_config(state: State<'_, AppState>, config: AppConfig) -> Result<bool, String> {
    // Delegate to ConfigService which updates both in-memory state and persistent file
    match state.app_services.as_ref() {
        Some(services) => {
            let status = services.config_service.update_config(config).await;
            // Log persistence status so users can see it in the UI log panel
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: if status.contains("failed") {
                        "warn".to_string()
                    } else {
                        "info".to_string()
                    },
                    message: status,
                    module: Some("config".to_string()),
                })
                .await;
            Ok(true)
        }
        None => {
            // Fallback: update in-memory only (no ConfigService without DB)
            {
                let mut app_config = state.config.write().await;
                *app_config = config;
            }
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warn".to_string(),
                    message: "Config updated in memory only (no persistence path)".to_string(),
                    module: Some("config".to_string()),
                })
                .await;
            Ok(true)
        }
    }
}

#[tauri::command]
pub async fn get_market_data(
    state: State<'_, AppState>,
    symbol: String,
) -> Result<MarketData, String> {
    // Route through the services layer so the command never touches the
    // data-source / infrastructure layer directly (layering + DIP).
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| "Market service not initialized".to_string())?;

    services
        .market_service
        .get_realtime_data(&symbol)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn submit_order(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    order: Order,
) -> Result<String, String> {
    // The order-placement pipeline (market data → risk check → submit →
    //   persist → emit → async execution) lives in `OrderProcessor` so the
    //   command stays a *thin adapter* (SRP) and never reaches into the
    //   domain / engine / infrastructure layers directly.
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| "Order service not initialized (no database connection)".to_string())?;

    let placement = services
        .order_processor
        .place_order(order)
        .await
        .map_err(|e| e.to_string())?;

    // Forward the UI event; the use-case already ran persistence + async execution.
    let _ = app.emit("order:submitted", placement.event);

    Ok(placement.order_id.to_string())
}

#[tauri::command]
pub async fn get_account_info(state: State<'_, AppState>) -> Result<Account, String> {
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_account_info().await {
            Ok(account) => {
                monitor_layer::MetricsCollector::set_account_balance(
                    account.total_assets.to_f64().unwrap_or(0.0),
                );
                monitor_layer::MetricsCollector::set_position_value(
                    account.market_value.to_f64().unwrap_or(0.0),
                );
                monitor_layer::MetricsCollector::set_daily_pnl(
                    account.daily_pnl.to_f64().unwrap_or(0.0),
                );
                Ok(account)
            }
            Err(service_error) => {
                let msg = format!("Account info unavailable: {}", service_error);
                state
                    .log_buffer
                    .add_entry(quant_common::types::LogEntry {
                        timestamp: Utc::now(),
                        level: "error".to_string(),
                        message: msg.clone(),
                        module: Some("commands".to_string()),
                    })
                    .await;
                Err(msg)
            }
        },
        None => Err("Account service not initialized (no database connection)".to_string()),
    }
}

#[tauri::command]
pub async fn get_positions(state: State<'_, AppState>) -> Result<Vec<Position>, String> {
    match state.app_services.as_ref() {
        Some(services) => match services.account_service.get_positions().await {
            Ok(positions) => Ok(positions),
            Err(e) => Err(format!("Positions unavailable: {}", e)),
        },
        None => Err("Account service not initialized (no database connection)".to_string()),
    }
}

#[tauri::command]
pub async fn get_active_orders(state: State<'_, AppState>) -> Result<Vec<Order>, String> {
    // 优先从 OrderManager 内存获取真实活跃订单（Submitted / PartiallyFilled）
    let manager_orders = state.order_manager.get_active_orders().await;
    if !manager_orders.is_empty() {
        return Ok(manager_orders);
    }

    // 内存无活跃订单时，降级查询数据库
    if let Some(services) = state.app_services.as_ref() {
        if let Ok(orders) = services.account_service.get_active_orders().await {
            return Ok(orders);
        }
    }

    // 无任何数据源可用 → 返回空列表（不再返回硬编码 mock 假数据）
    Ok(Vec::new())
}
