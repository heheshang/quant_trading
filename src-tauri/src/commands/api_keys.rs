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
) -> Result<bool, String> {
    state.require_role("admin").await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    services
        .api_key_service
        .save_api_key(user_id, &exchange, &api_key, &secret, passphrase)
        .await
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// 获取某用户已保存的 API 密钥（脱敏，无明文 secret）。
#[tauri::command]
pub async fn get_api_keys(
    state: State<'_, AppState>,
    user_id: i64,
) -> Result<Vec<MaskedApiKey>, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    services
        .api_key_service
        .get_api_keys(user_id)
        .await
        .map_err(|e| e.to_string())
}
