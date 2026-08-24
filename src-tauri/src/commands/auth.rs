use crate::state::AppState;
use crate::state::AuthedUser;
use tauri::State;

/// Decode a JWT token with the configured secret to recover the subject claims
/// (user_id, username, role) used to (re)establish the in-memory auth session.
async fn token_claims(state: &AppState, token: &str) -> Option<security::Claims> {
    let config = state.config.read().await;
    let auth = security::AuthService::new(
        config.security.jwt_secret.clone(),
        config.security.token_expiry_hours as i64,
    );
    auth.verify_token(token).ok()
}

fn session_from_claims(claims: &security::Claims) -> AuthedUser {
    let role = claims
        .roles
        .first()
        .cloned()
        .unwrap_or_else(|| "trader".to_string());
    AuthedUser {
        user_id: claims.sub,
        username: claims.username.clone(),
        role,
    }
}

/// 用户登录
#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    username: String,
    password: String,
    code: Option<String>,
) -> Result<String, String> {
    // Delegate to AuthService which verifies against database password hashes.
    // When the user has 2FA enabled, `code` is verified by the service before
    // a token is issued.
    let result = match state.app_services.as_ref() {
        Some(services) => services
            .auth_service
            .login(&username, &password, code.as_deref())
            .await
            .map_err(|e| e.to_string()),
        None => {
            // Fallback only when no database — never hardcode credentials
            Err("Authentication unavailable: no database connection".to_string())
        }
    };
    // Establish the in-memory auth session on success so protected commands
    // can enforce RBAC (`require_auth` / `require_role`).
    if let Ok(token) = &result {
        if let Some(claims) = token_claims(&state, token).await {
            *state.auth_session.write().await = Some(session_from_claims(&claims));
        }
    }
    let success = result.is_ok();
    // Resolve the real user id from the username so even a failed login with an
    // existing account attributes the event to that user. A non-existent
    // username resolves to None; the login itself already failed.
    let resolved_user_id = match state.app_services.as_ref() {
        Some(services) => services
            .auth_service
            .resolve_user_id(&username)
            .await
            .unwrap_or(None),
        None => None,
    };
    let audit_user_id = resolved_user_id.map(|id| id.to_string()).unwrap_or_default();
    let _ = state
        .audit_logger
        .log_login(&audit_user_id, &username, None, success)
        .await;
    result
}

/// 验证 Token
#[tauri::command]
pub async fn verify_token(state: State<'_, AppState>, token: String) -> Result<bool, String> {
    let valid = match state.app_services.as_ref() {
        Some(services) => services.auth_service.verify_token(&token).await,
        None => {
            let config = state.config.read().await;
            let auth_service = security::AuthService::new(
                config.security.jwt_secret.clone(),
                config.security.token_expiry_hours as i64,
            );
            auth_service.verify_token(&token).is_ok()
        }
    };
    // Re-validate & refresh the in-memory session from a valid token so a
    // restored session (e.g. `restoreSession` on app start) re-establishes
    // RBAC state. Invalid tokens leave the session as-is.
    if valid {
        if let Some(claims) = token_claims(&state, &token).await {
            *state.auth_session.write().await = Some(session_from_claims(&claims));
        }
    }
    Ok(valid)
}

/// 更新用户资料
#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    profile_data: serde_json::Value,
) -> Result<bool, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    // Use the username from the profile data, or default to "admin"
    let username = profile_data
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("admin");

    let result = services
        .auth_service
        .update_profile(username, &profile_data)
        .await
        .map_err(|e| e.to_string());
    let success = result.is_ok();
    let _ = state
        .audit_logger
        .log(
            "0",
            username,
            security::audit::AuditAction::ConfigChange,
            "user_profile",
            profile_data.clone(),
            None,
            success,
            if success {
                None
            } else {
                result.as_ref().err().cloned()
            },
        )
        .await;
    result
}

/// 修改密码
#[tauri::command]
pub async fn change_password(
    state: State<'_, AppState>,
    current_password: String,
    new_password: String,
    username: Option<String>,
) -> Result<bool, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let username = username.unwrap_or_else(|| "admin".to_string());

    let result = services
        .auth_service
        .change_password(&username, &current_password, &new_password)
        .await
        .map_err(|e| e.to_string());
    let success = result.is_ok();
    let _ = state
        .audit_logger
        .log(
            "0",
            &username,
            security::audit::AuditAction::ConfigChange,
            "password",
            serde_json::json!({}),
            None,
            success,
            if success {
                None
            } else {
                result.as_ref().err().cloned()
            },
        )
        .await;
    result
}

/// 获取用户资料
#[tauri::command]
pub async fn get_user_profile(
    state: State<'_, AppState>,
    username: Option<String>,
) -> Result<serde_json::Value, String> {
    state.require_auth().await?;
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;
    let username = username.unwrap_or_else(|| "admin".to_string());
    services
        .auth_service
        .get_user_profile(&username)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use security::Claims;

    #[test]
    fn session_from_claims_maps_subject_username_and_role() {
        let claims = Claims {
            sub: 42,
            username: "operator".to_string(),
            exp: 0,
            iat: 0,
            jti: "jti".to_string(),
            roles: vec!["trader".to_string()],
            version: 0,
        };
        let session = session_from_claims(&claims);
        assert_eq!(session.user_id, 42);
        assert_eq!(session.username, "operator");
        assert_eq!(session.role, "trader");
    }

    #[test]
    fn session_from_claims_falls_back_to_trader_when_no_role() {
        let claims = Claims {
            sub: 7,
            username: "solo".to_string(),
            exp: 0,
            iat: 0,
            jti: "jti".to_string(),
            roles: vec![],
            version: 0,
        };
        let session = session_from_claims(&claims);
        assert_eq!(session.role, "trader");
    }
}
