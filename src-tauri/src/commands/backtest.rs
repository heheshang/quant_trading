use crate::state::AppState;
use quant_common::types::{Alert, BacktestResult};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
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
    timeframe: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<BacktestResult>> {
    use crate::commands::not_init_err;
    use quant_common::api::{ok_result, ApiFailure};
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;

    let start = chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", start_date))
        .map_err(|e| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid start date: {}", e)))?
        .with_timezone(&chrono::Utc);
    let end = chrono::DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", end_date))
        .map_err(|e| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid end date: {}", e)))?
        .with_timezone(&chrono::Utc);
    let init_cap = rust_decimal::Decimal::from_f64(initial_capital)
        .ok_or_else(|| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid initial capital: {}", initial_capital)))?;
    let comm_rate = rust_decimal::Decimal::from_f64(commission_rate)
        .ok_or_else(|| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid commission rate: {}", commission_rate)))?;
    let slip = rust_decimal::Decimal::from_f64(slippage)
        .ok_or_else(|| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid slippage: {}", slippage)))?;

    let data = services
        .strategy_service
        .run_backtest(&strategy_id, start, end, init_cap, comm_rate, slip, &symbols, &timeframe)
        .await?;
    ok_result(data)
}

/// 查询回测结果列表（分页）
#[tauri::command]
pub async fn get_backtest_results(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<data_layer::BacktestResultsPage>> {
    use crate::commands::not_init_err;
    use quant_common::api::ok_result;
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services.strategy_service.get_backtest_results(limit, offset).await?;
    ok_result(data)
}

/// 查询单个回测结果详情（含 equity_curve）
#[tauri::command]
pub async fn get_backtest_result(
    state: State<'_, AppState>,
    id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<BacktestResult>> {
    use crate::commands::not_init_err;
    use quant_common::api::{ok_result, ApiFailure};
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let id_num: i64 = id.parse().map_err(|e| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid ID: {}", e)))?;
    let data = services.strategy_service.get_backtest_result(id_num).await?;
    ok_result(data)
}

/// 删除回测结果
#[tauri::command]
pub async fn delete_backtest_result(
    state: State<'_, AppState>,
    id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use crate::commands::not_init_err;
    use quant_common::api::{ok_result, ApiFailure};
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let id_num: i64 = id.parse().map_err(|e| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid ID: {}", e)))?;
    let data = services.strategy_service.delete_backtest_result(id_num).await?;
    ok_result(data)
}

/// 获取实时指标数据（Monitor UI）。
///
/// 订单数为数据库累计（含历史），账户指标取真实 account_info；DB 不可用时降级到原子值。
#[tauri::command]
pub async fn get_metrics(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<HashMap<String, f64>>> {
    use quant_common::api::ok_result;
    let mut metrics = HashMap::new();

    match state.app_services.as_ref() {
        Some(services) => {
            match services.account_service.get_order_counts().await {
                Ok(c) => {
                    metrics.insert("orders_total".to_string(), c.total as f64);
                    metrics.insert("orders_filled".to_string(), c.filled as f64);
                    metrics.insert("orders_cancelled".to_string(), c.cancelled as f64);
                    metrics.insert("orders_rejected".to_string(), c.rejected as f64);
                    metrics.insert("orders_open".to_string(), c.open as f64);
                }
                Err(_) => {
                    metrics.insert("orders_total".to_string(), monitor_layer::ORDERS_TOTAL.get());
                    metrics.insert("orders_filled".to_string(), monitor_layer::ORDERS_FILLED.get());
                    metrics.insert("orders_cancelled".to_string(), monitor_layer::ORDERS_CANCELLED.get());
                    metrics.insert("orders_rejected".to_string(), monitor_layer::ORDERS_REJECTED.get());
                }
            }
            match services.account_service.get_latest_equity("USDT").await {
                Ok(Some(equity)) => {
                    metrics.insert("account_balance".to_string(), equity.to_f64().unwrap_or(0.0));
                    metrics.insert("position_value".to_string(), equity.to_f64().unwrap_or(0.0));
                }
                _ => {
                    let fallback = services.account_service.get_account_info().await.map(|a| a.total_assets.to_f64().unwrap_or(0.0)).unwrap_or(monitor_layer::ACCOUNT_BALANCE.get());
                    metrics.insert("account_balance".to_string(), fallback);
                    metrics.insert("position_value".to_string(), fallback);
                }
            }
            match services.account_service.get_today_equity_pnl("USDT").await {
                Ok(pnl) => {
                    metrics.insert("daily_pnl".to_string(), pnl.to_f64().unwrap_or(0.0));
                }
                Err(_) => {
                    let fallback = services.account_service.get_account_info().await.map(|a| a.daily_pnl.to_f64().unwrap_or(0.0)).unwrap_or(monitor_layer::DAILY_PNL.get());
                    metrics.insert("daily_pnl".to_string(), fallback);
                }
            }
        }
        None => {
            metrics.insert("orders_total".to_string(), monitor_layer::ORDERS_TOTAL.get());
            metrics.insert("orders_filled".to_string(), monitor_layer::ORDERS_FILLED.get());
            metrics.insert("orders_cancelled".to_string(), monitor_layer::ORDERS_CANCELLED.get());
            metrics.insert("orders_rejected".to_string(), monitor_layer::ORDERS_REJECTED.get());
            metrics.insert("account_balance".to_string(), monitor_layer::ACCOUNT_BALANCE.get());
            metrics.insert("position_value".to_string(), monitor_layer::POSITION_VALUE.get());
            metrics.insert("daily_pnl".to_string(), monitor_layer::DAILY_PNL.get());
        }
    }

    ok_result(metrics)
}

/// 获取告警信息
#[tauri::command]
pub async fn get_alerts(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<Alert>>> {
    use quant_common::api::ok_result;
    let alerts = state.alert_manager.get_alerts().await;
    ok_result(alerts)
}

/// 确认告警
#[tauri::command]
pub async fn acknowledge_alert(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    alert_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use quant_common::api::{ok_result, ApiFailure};
    let alert_id_num: i64 = alert_id
        .parse()
        .map_err(|e| ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid alert ID: {}", e)))?;

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
    ok_result(acknowledged)
}

/// 获取日志信息
#[tauri::command]
pub async fn get_logs(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    level: Option<String>,
    limit: Option<u32>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<quant_common::types::LogEntry>>> {
    use quant_common::api::ok_result;
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
    ok_result(logs)
}

/// 检查 Redis 连接状态
#[tauri::command]
pub async fn check_redis_status(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use quant_common::api::{ok_result, ApiFailure};
    match state.redis_cache.as_ref() {
        Some(cache) => {
            let ok = cache.health_check().await.map_err(|e| ApiFailure::new(quant_common::api::code::DATABASE, format!("Redis health check failed: {}", e)))?;
            ok_result(ok)
        }
        None => Err(ApiFailure::new(
            quant_common::api::code::NOT_INITIALIZED,
            "Redis client not initialized".to_string(),
        )),
    }
}
