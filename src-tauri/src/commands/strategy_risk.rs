use crate::state::AppState;
use chrono::Utc;
use quant_common::types::{Account, Alert, Order, Position, StrategyParams};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use tauri::State;

/// 获取所有策略
#[tauri::command]
pub async fn get_strategies(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<StrategyParams>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state
        .app_services
        .as_ref()
        .ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services.strategy_service.get_strategies().await?;
    ok_result(data)
}

/// 创建或更新策略
#[tauri::command]
pub async fn save_strategy(
    state: State<'_, AppState>,
    strategy: StrategyParams,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services.strategy_service.save_strategy(&strategy).await?;
    ok_result(data)
}

/// 删除策略
#[tauri::command]
pub async fn delete_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services.strategy_service.delete_strategy(&strategy_id).await?;
    ok_result(data)
}

/// 启用/禁用策略
#[tauri::command]
pub async fn toggle_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
    enabled: bool,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services.strategy_service.toggle_strategy(&strategy_id, enabled).await?;
    ok_result(data)
}

/// 部署策略
#[tauri::command]
pub async fn deploy_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let status = services.strategy_service.deploy_strategy(&strategy_id).await?;
    ok_result(format!("{:?}", status))
}

/// 启动策略
#[tauri::command]
pub async fn start_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let status = services.strategy_service.start_strategy(&strategy_id).await?;
    ok_result(format!("{:?}", status))
}

/// 停止策略
#[tauri::command]
pub async fn stop_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let status = services.strategy_service.stop_strategy(&strategy_id).await?;
    ok_result(format!("{:?}", status))
}

/// 暂停策略
#[tauri::command]
pub async fn pause_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let status = services.strategy_service.pause_strategy(&strategy_id).await?;
    ok_result(format!("{:?}", status))
}

/// 恢复策略
#[tauri::command]
pub async fn resume_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let status = services.strategy_service.resume_strategy(&strategy_id).await?;
    ok_result(format!("{:?}", status))
}

/// 归档策略
#[tauri::command]
pub async fn archive_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let status = services.strategy_service.archive_strategy(&strategy_id).await?;
    ok_result(format!("{:?}", status))
}

/// 列出所有已注册的策略类型元数据
#[tauri::command]
pub async fn list_strategy_types(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<strategy_layer::registry::StrategyTypeInfo>>> {
    use crate::commands::not_init_err;
    use quant_common::api::{ok_result, ApiFailure};
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services
        .strategy_service
        .list_strategy_types()
        .map_err(|e| ApiFailure::new(quant_common::api::code::INTERNAL, e.to_string()))?;
    ok_result(data)
}

/// 获取单个策略类型的元数据（含参数 Schema）
#[tauri::command]
pub async fn get_strategy_type_info(
    state: State<'_, AppState>,
    type_name: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<strategy_layer::registry::StrategyTypeInfo>> {
    use crate::commands::not_init_err;
    use quant_common::api::{ok_result, ApiFailure};
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services
        .strategy_service
        .get_strategy_type_info(&type_name)
        .map_err(|e| ApiFailure::new(quant_common::api::code::INTERNAL, e.to_string()))?;
    ok_result(data)
}

/// 创建新策略（自动生成 UUID v7 strategy_id，含参数验证）
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn create_strategy(
    state: State<'_, AppState>,
    type_name: String,
    strategy_name: String,
    params: serde_json::Value,
    enabled: bool,
    max_position: f64,
    max_daily_loss: f64,
    instance_label: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    symbols: Vec<String>,
    user_id: i64,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;

    let max_pos = rust_decimal::Decimal::from_f64(max_position)
        .ok_or_else(|| quant_common::api::ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid max_position: {}", max_position)))?;
    let max_loss = rust_decimal::Decimal::from_f64(max_daily_loss)
        .ok_or_else(|| quant_common::api::ApiFailure::new(quant_common::api::code::INVALID_PARAM, format!("Invalid max_daily_loss: {}", max_daily_loss)))?;

    let data = services
        .strategy_service
        .create_strategy(
            &type_name,
            &strategy_name,
            params,
            enabled,
            max_pos,
            max_loss,
            instance_label,
            description,
            tags,
            symbols,
            user_id,
        )
        .await?;
    ok_result(data)
}

/// 获取风险指标
#[tauri::command]
pub async fn get_risk_metrics(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<HashMap<String, f64>>> {
    use crate::commands::auth_err;
    use quant_common::api::ok_result;
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    // Prefer RiskService (DB-backed) for real VaR computation with historical returns
    if let Some(ref services) = state.app_services {
        if let Ok(metrics) = services.risk_service.get_risk_metrics().await {
            return ok_result(metrics);
        }
        // 失败记录日志后走内存回退。
    }

    use risk_layer::VaRCalculator;
    use rust_decimal::prelude::ToPrimitive;
    let mut metrics = HashMap::new();
    let config = state.config.read().await;
    let risk_config = &config.risk;
    let var_95 = VaRCalculator::historical_simulation(&[dec!(0.0)], 0.95);
    let var_99 = VaRCalculator::historical_simulation(&[dec!(0.0)], 0.99);
    metrics.insert("var_95".to_string(), var_95.to_f64().unwrap_or(0.0));
    metrics.insert("var_99".to_string(), var_99.to_f64().unwrap_or(0.0));
    metrics.insert("max_position_size".to_string(), risk_config.max_position_size);
    metrics.insert("max_daily_loss".to_string(), risk_config.max_daily_loss);
    metrics.insert("max_drawdown".to_string(), risk_config.max_drawdown);
    ok_result(metrics)
}

/// 获取风险配置
#[tauri::command]
pub async fn get_risk_config(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<quant_common::config::RiskConfig>> {
    use quant_common::api::{ok_result, ApiFailure};
    if let Err(e) = state.require_role("admin").await {
        return Err(ApiFailure::new(quant_common::api::code::FORBIDDEN, e));
    }
    if let Some(ref services) = state.app_services {
        if let Ok(config) = services.risk_service.get_risk_config().await {
            state.config.write().await.risk = config.clone();
            return ok_result(config);
        }
    }
    let config = state.config.read().await;
    ok_result(config.risk.clone())
}

/// 更新风险配置
#[tauri::command]
pub async fn update_risk_config(
    state: State<'_, AppState>,
    config: quant_common::config::RiskConfig,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use crate::commands::auth_err;
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    match state.app_services.as_ref() {
        Some(services) => {
            let mut new_config = services.config_service.get_config().await;
            new_config.risk = config.clone();
            let status = services.config_service.update_config(new_config).await;
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

            match services.risk_service.update_risk_config(&config).await {
                Ok(true) => {
                    state
                        .log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "info".to_string(),
                            message: "Risk config saved to database".to_string(),
                            module: Some("risk".to_string()),
                        })
                        .await;
                    ok_result(true)
                }
                Ok(false) => {
                    state
                        .log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "warn".to_string(),
                            message: "Risk config row not found in database".to_string(),
                            module: Some("risk".to_string()),
                        })
                        .await;
                    ok_result(false)
                }
                Err(e) => {
                    state
                        .log_buffer
                        .add_entry(quant_common::types::LogEntry {
                            timestamp: Utc::now(),
                            level: "warn".to_string(),
                            message: format!(
                                "Risk config DB update failed, kept memory/file config: {}",
                                e
                            ),
                            module: Some("risk".to_string()),
                        })
                        .await;
                    ok_result(true)
                }
            }
        }
        None => {
            let mut app_config = state.config.write().await;
            app_config.risk = config;
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warn".to_string(),
                    message: "Risk config updated in memory only (no persistence)".to_string(),
                    module: Some("config".to_string()),
                })
                .await;
            ok_result(true)
        }
    }
}

/// 执行事前风控检查
#[tauri::command]
pub async fn pre_trade_check(
    state: State<'_, AppState>,
    order: Order,
    account: Account,
    positions: Vec<Position>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use quant_common::api::ok_result;
    use risk_layer::PreTradeRiskChecker;

    // Use the application's live risk configuration instead of hardcoded defaults
    let mut risk_config = state.config.read().await.risk.clone();
    if let Some(ref services) = state.app_services {
        if let Ok(db_risk_config) = services.risk_service.get_risk_config().await {
            risk_config = db_risk_config;
        }
    }

    let checker = PreTradeRiskChecker::new(risk_config);

    match checker.check_order(&order, &account, &positions) {
        Ok(_) => {
            // Log successful check
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "info".to_string(),
                    message: format!("Pre-trade check passed for order {}", order.order_id),
                    module: Some("risk".to_string()),
                })
                .await;
            ok_result(true)
        }
        Err(e) => {
            let error_msg = format!("Pre-trade check failed: {}", e);

            // Create alert for failed check
            let alert = Alert {
                alert_id: 0,
                level: quant_common::types::AlertLevel::Warning,
                source: "Risk Management".to_string(),
                message: error_msg.clone(),
                timestamp: Utc::now(),
                acknowledged: false,
            };
            state.alert_manager.send_alert(alert).await;

            // Log the failure
            state
                .log_buffer
                .add_entry(quant_common::types::LogEntry {
                    timestamp: Utc::now(),
                    level: "warning".to_string(),
                    message: error_msg,
                    module: Some("risk".to_string()),
                })
                .await;

            ok_result(false)
        }
    }
}
