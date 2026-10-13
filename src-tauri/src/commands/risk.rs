//! 风控命令
//!
//! 处理风险指标、风险配置和事前风控检查命令。

use crate::state::AppState;
use chrono::Utc;
use quant_common::types::{Account, Alert, Order, Position};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use tauri::State;

// ── 认证辅助宏 ──────────────────────────────────────────────────────────────

macro_rules! require_auth {
    ($state:expr) => {
        $state.require_auth().await.map_err(crate::commands::auth_err)?
    };
}

macro_rules! require_admin {
    ($state:expr) => {
        $state.require_role("admin").await.map_err(crate::commands::auth_err)?
    };
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 获取有效的风险配置（优先从数据库读取，否则从内存读取）
async fn effective_risk_config(
    state: &AppState,
) -> quant_services::ServiceResult<quant_common::config::RiskConfig> {
    if let Some(services) = &state.app_services {
        let mut app_config = state.config.write().await;
        match services.risk_service.get_risk_config().await {
            Ok(config) => {
                app_config.risk = config.clone();
                return Ok(config);
            }
            Err(quant_services::ServiceError::DatabaseNotConnected) => {
                return Ok(app_config.risk.clone());
            }
            Err(error) => return Err(error),
        }
    }
    Ok(state.config.read().await.risk.clone())
}

// ── 风控命令 ────────────────────────────────────────────────────────────────

/// 获取风险指标
#[tauri::command]
pub async fn get_risk_metrics(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<HashMap<String, f64>>> {
    use quant_common::api::ok_result;
    let _user = require_auth!(&state);

    // 优先使用 RiskService（数据库支持）进行真实的 VaR 计算
    if let Some(ref services) = state.app_services {
        if let Ok(metrics) = services.risk_service.get_risk_metrics().await {
            return ok_result(metrics);
        }
    }

    // 降级到内存计算
    use risk_layer::VaRCalculator;
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
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<quant_common::config::RiskConfig>>
{
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    ok_result(effective_risk_config(&state).await?)
}

/// 更新风险配置
#[tauri::command]
pub async fn update_risk_config(
    state: State<'_, AppState>,
    config: quant_common::config::RiskConfig,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);

    let services = state
        .app_services
        .as_ref()
        .ok_or(quant_services::ServiceError::DatabaseNotConnected)?;

    let mut app_config = state.config.write().await;
    services.risk_service.update_risk_config(&config).await?;
    app_config.risk = config;
    drop(app_config);

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

    let risk_config = effective_risk_config(&state).await?;
    let checker = PreTradeRiskChecker::new(risk_config);

    match checker.check_order(&order, &account, &positions) {
        Ok(_) => {
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

            // 创建告警
            let alert = Alert {
                alert_id: 0,
                level: quant_common::types::AlertLevel::Warning,
                source: "Risk Management".to_string(),
                message: error_msg.clone(),
                timestamp: Utc::now(),
                acknowledged: false,
            };
            state.alert_manager.send_alert(alert).await;

            // 记录日志
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
