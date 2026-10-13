//! 策略管理命令
//!
//! 处理策略的 CRUD 和生命周期管理命令。

use crate::state::AppState;
use quant_common::types::StrategyParams;
use rust_decimal::Decimal;
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

macro_rules! get_services {
    ($state:expr) => {
        $state
            .app_services
            .as_ref()
            .ok_or_else(|| crate::commands::not_init_err("应用服务未初始化"))?
    };
}

// ── 策略查询命令 ────────────────────────────────────────────────────────────

/// 获取所有策略
#[tauri::command]
pub async fn get_strategies(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<StrategyParams>>> {
    use quant_common::api::ok_result;
    let _user = require_auth!(&state);
    let services = get_services!(&state);
    let data = services.strategy_service.get_strategies().await?;
    ok_result(data)
}

// ── 策略写入命令 ────────────────────────────────────────────────────────────

/// 创建或更新策略
#[tauri::command]
pub async fn save_strategy(
    state: State<'_, AppState>,
    strategy: StrategyParams,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let data = services.strategy_service.save_strategy(&strategy).await?;
    ok_result(data)
}

/// 删除策略
#[tauri::command]
pub async fn delete_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let data = services
        .strategy_service
        .delete_strategy(&strategy_id)
        .await?;
    ok_result(data)
}

/// 启用/禁用策略
#[tauri::command]
pub async fn toggle_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
    enabled: bool,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let data = services
        .strategy_service
        .toggle_strategy(&strategy_id, enabled)
        .await?;
    ok_result(data)
}

// ── 策略生命周期命令 ────────────────────────────────────────────────────────

/// 部署策略
#[tauri::command]
pub async fn deploy_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let status = services
        .strategy_service
        .deploy_strategy(&strategy_id)
        .await?;
    ok_result(format!("{:?}", status))
}

/// 启动策略
#[tauri::command]
pub async fn start_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let status = services
        .strategy_service
        .start_strategy(&strategy_id)
        .await?;
    ok_result(format!("{:?}", status))
}

/// 停止策略
#[tauri::command]
pub async fn stop_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let status = services
        .strategy_service
        .stop_strategy(&strategy_id)
        .await?;
    ok_result(format!("{:?}", status))
}

/// 暂停策略
#[tauri::command]
pub async fn pause_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let status = services
        .strategy_service
        .pause_strategy(&strategy_id)
        .await?;
    ok_result(format!("{:?}", status))
}

/// 恢复策略
#[tauri::command]
pub async fn resume_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let status = services
        .strategy_service
        .resume_strategy(&strategy_id)
        .await?;
    ok_result(format!("{:?}", status))
}

/// 归档策略
#[tauri::command]
pub async fn archive_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<String>> {
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);
    let status = services
        .strategy_service
        .archive_strategy(&strategy_id)
        .await?;
    ok_result(format!("{:?}", status))
}

// ── 策略类型元数据命令 ──────────────────────────────────────────────────────

/// 列出所有已注册的策略类型元数据
#[tauri::command]
pub async fn list_strategy_types(
    state: State<'_, AppState>,
) -> quant_common::api::ApiResult<
    quant_common::api::ApiResponse<Vec<strategy_layer::registry::StrategyTypeInfo>>,
> {
    use quant_common::api::{ok_result, ApiFailure};
    let services = get_services!(&state);
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
) -> quant_common::api::ApiResult<
    quant_common::api::ApiResponse<strategy_layer::registry::StrategyTypeInfo>,
> {
    use quant_common::api::{ok_result, ApiFailure};
    let services = get_services!(&state);
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
    use quant_common::api::ok_result;
    let _user = require_admin!(&state);
    let services = get_services!(&state);

    let max_pos = Decimal::try_from(max_position).map_err(|_| {
        quant_common::api::ApiFailure::new(
            quant_common::api::code::INVALID_PARAM,
            format!("Invalid max_position: {}", max_position),
        )
    })?;
    let max_loss = Decimal::try_from(max_daily_loss).map_err(|_| {
        quant_common::api::ApiFailure::new(
            quant_common::api::code::INVALID_PARAM,
            format!("Invalid max_daily_loss: {}", max_daily_loss),
        )
    })?;

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
