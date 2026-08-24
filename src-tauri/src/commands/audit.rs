use crate::state::AppState;
use security::AuditLog;
use tauri::State;

/// 查询审计日志（分页 + 按用户 / action 过滤）。
///
/// `user_id` 可选；`action` 传入如 `"Login"`、`"OrderSubmit"`；
/// `limit`/`offset` 为分页参数。
#[tauri::command]
pub async fn get_audit_logs(
    state: State<'_, AppState>,
    user_id: Option<i64>,
    username: Option<String>,
    action: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<AuditLog>, String> {
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);

    state
        .audit_logger
        .query_logs(user_id, username, action, limit, offset)
        .await
        .map_err(|e| e.to_string())
}
