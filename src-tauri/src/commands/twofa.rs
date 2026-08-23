use crate::state::AppState;
use tauri::State;

/// 开启双因素认证（TOTP）流程：为用户生成并持久化一个待验证的 TOTP 密钥，
/// 返回密钥 + 加密密钥 + otpauth URI（供前端生成二维码）。
#[tauri::command]
pub async fn enable_2fa(
    state: State<'_, AppState>,
    user_id: i64,
) -> Result<quant_services::Enable2faResult, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .auth_service
        .enable_2fa(user_id)
        .await
        .map_err(|e| e.to_string())
}

/// 校验用户输入的 6 位 TOTP 动态验证码；校验通过则把账户标记为「已启用 2FA」。
///
/// 返回 `true` 表示校验通过并已启用，`false` 表示验证码错误（可重试）。
#[tauri::command]
pub async fn verify_2fa_code(
    state: State<'_, AppState>,
    user_id: i64,
    code: String,
) -> Result<bool, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .auth_service
        .verify_2fa_code(user_id, &code)
        .await
        .map_err(|e| e.to_string())
}

/// 关闭双因素认证。**只有**在提供的动态验证码校验通过后才会真正禁用；
/// 验证码错误时返回 `false`（拒绝禁用）。
#[tauri::command]
pub async fn disable_2fa(
    state: State<'_, AppState>,
    user_id: i64,
    code: String,
) -> Result<bool, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    services
        .auth_service
        .disable_2fa(user_id, &code)
        .await
        .map_err(|e| e.to_string())
}
