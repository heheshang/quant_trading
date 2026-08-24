use crate::state::AppState;
use quant_services::MaskedApiKey;
use tauri::State;

/// 保存交易所 API 密钥（加密后落库，不返回明文 secret）。
#[tauri::command]
pub async fn save_api_key(
    state: State<'_, AppState>,
    user_id: i64,
    exchange: String,
    api_key: String,
    secret: String,
    passphrase: Option<String>,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<bool>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_role("admin").await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    services
        .api_key_service
        .save_api_key(user_id, &exchange, &api_key, &secret, passphrase)
        .await?;
    ok_result(true)
}

/// 获取某用户已保存的 API 密钥（脱敏，无明文 secret）。
#[tauri::command]
pub async fn get_api_keys(
    state: State<'_, AppState>,
    user_id: i64,
) -> quant_common::api::ApiResult<quant_common::api::ApiResponse<Vec<MaskedApiKey>>> {
    use crate::commands::{auth_err, not_init_err};
    use quant_common::api::ok_result;
    if let Err(e) = state.require_auth().await {
        return Err(auth_err(e));
    }
    let services = state.app_services.as_ref().ok_or_else(|| not_init_err("应用服务未初始化"))?;
    let data = services.api_key_service.get_api_keys(user_id).await?;
    ok_result(data)
}
