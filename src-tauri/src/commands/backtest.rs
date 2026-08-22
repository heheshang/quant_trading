use crate::state::AppState;
use quant_common::types::{Alert, BacktestResult};
use rust_decimal::prelude::FromPrimitive;
use std::collections::HashMap;
use tauri::{Emitter, State};

#[tauri::command]
pub async fn run_backtest(
    state: State<'_, AppState>,
    strategy_id: String,
    start_date: String,
    end_date: String,
    initial_capital: f64,
    commission_rate: f64,
    slippage: f64,
    symbols: Vec<String>,
) -> Result<BacktestResult, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    let start = chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", start_date))
        .map_err(|e| format!("Invalid start date: {}", e))?
        .with_timezone(&chrono::Utc);

    let end = chrono::DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", end_date))
        .map_err(|e| format!("Invalid end date: {}", e))?
        .with_timezone(&chrono::Utc);

    let init_cap = rust_decimal::Decimal::from_f64(initial_capital)
        .ok_or_else(|| format!("Invalid initial capital: {}", initial_capital))?;
    let comm_rate = rust_decimal::Decimal::from_f64(commission_rate)
        .ok_or_else(|| format!("Invalid commission rate: {}", commission_rate))?;
    let slip = rust_decimal::Decimal::from_f64(slippage)
        .ok_or_else(|| format!("Invalid slippage: {}", slippage))?;

    services
        .strategy_service
        .run_backtest(
            &strategy_id,
            start,
            end,
            init_cap,
            comm_rate,
            slip,
            &symbols,
        )
        .await
        .map_err(|e| e.to_string())
}

/// 查询回测结果列表（分页）
#[tauri::command]
pub async fn get_backtest_results(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<quant_repository::BacktestResultSummaryRow>, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    services
        .strategy_service
        .get_backtest_results(limit, offset)
        .await
        .map_err(|e| e.to_string())
}

/// 查询单个回测结果详情（含 equity_curve）
#[tauri::command]
pub async fn get_backtest_result(
    state: State<'_, AppState>,
    id: String,
) -> Result<BacktestResult, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    let id_num: i64 = id.parse().map_err(|e| format!("Invalid ID: {}", e))?;

    services
        .strategy_service
        .get_backtest_result(id_num)
        .await
        .map_err(|e| e.to_string())
}

/// 删除回测结果
#[tauri::command]
pub async fn delete_backtest_result(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    let id_num: i64 = id.parse().map_err(|e| format!("Invalid ID: {}", e))?;

    services
        .strategy_service
        .delete_backtest_result(id_num)
        .await
        .map_err(|e| e.to_string())
}

/// 获取实时指标数据
#[tauri::command]
pub async fn get_metrics() -> Result<HashMap<String, f64>, String> {
    let mut metrics = HashMap::new();

    metrics.insert(
        "orders_total".to_string(),
        monitor_layer::ORDERS_TOTAL.get(),
    );
    metrics.insert(
        "orders_filled".to_string(),
        monitor_layer::ORDERS_FILLED.get(),
    );
    metrics.insert(
        "orders_cancelled".to_string(),
        monitor_layer::ORDERS_CANCELLED.get(),
    );
    metrics.insert(
        "account_balance".to_string(),
        monitor_layer::ACCOUNT_BALANCE.get(),
    );
    metrics.insert(
        "position_value".to_string(),
        monitor_layer::POSITION_VALUE.get(),
    );
    metrics.insert("daily_pnl".to_string(), monitor_layer::DAILY_PNL.get());

    Ok(metrics)
}

/// 获取告警信息
#[tauri::command]
pub async fn get_alerts(state: State<'_, AppState>) -> Result<Vec<Alert>, String> {
    let alerts = state.alert_manager.get_alerts().await;
    Ok(alerts)
}

/// 确认告警
#[tauri::command]
pub async fn acknowledge_alert(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    alert_id: String,
) -> Result<bool, String> {
    let alert_id_num: i64 = alert_id
        .parse()
        .map_err(|e| format!("Invalid alert ID: {}", e))?;

    let acknowledged = state.alert_manager.acknowledge_alert(alert_id_num).await;
    if acknowledged {
        let _ = app.emit(
            "ws:alerts",
            serde_json::json!({
                "type": "acknowledged",
                "alert_id": alert_id_num,
            }),
        );
    }
    Ok(acknowledged)
}

/// 获取日志信息
#[tauri::command]
pub async fn get_logs(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    level: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<quant_common::types::LogEntry>, String> {
    let logs = if let Some(level_filter) = level {
        state.log_buffer.get_entries_by_level(&level_filter).await
    } else {
        state.log_buffer.get_entries().await
    };

    let logs = if let Some(n) = limit {
        let n = n as usize;
        if logs.len() > n {
            logs[logs.len() - n..].to_vec()
        } else {
            logs
        }
    } else {
        logs
    };

    let _ = app.emit("ws:logs", &logs);
    Ok(logs)
}

/// 检查 Redis 连接状态
#[tauri::command]
pub async fn check_redis_status(state: State<'_, AppState>) -> Result<bool, String> {
    match state.redis_cache.as_ref() {
        Some(cache) => cache
            .health_check()
            .await
            .map_err(|e| format!("Redis health check failed: {}", e)),
        None => Err("Redis client not initialized".to_string()),
    }
}
