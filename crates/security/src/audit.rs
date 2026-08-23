use chrono::{DateTime, Utc};
use quant_common::Result;
use quant_repository::{AuditFilter, AuditLogRecord, AuditRepository, NewAuditLog};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// 审计日志类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    Login,
    Logout,
    OrderSubmit,
    OrderCancel,
    StrategyStart,
    StrategyStop,
    ConfigChange,
    ApiKeyAccess,
    DataExport,
    SystemShutdown,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub username: String,
    pub action: AuditAction,
    pub resource: String,
    pub details: serde_json::Value,
    pub ip_address: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

fn action_to_str(action: &AuditAction) -> String {
    match action {
        AuditAction::Login => "Login",
        AuditAction::Logout => "Logout",
        AuditAction::OrderSubmit => "OrderSubmit",
        AuditAction::OrderCancel => "OrderCancel",
        AuditAction::StrategyStart => "StrategyStart",
        AuditAction::StrategyStop => "StrategyStop",
        AuditAction::ConfigChange => "ConfigChange",
        AuditAction::ApiKeyAccess => "ApiKeyAccess",
        AuditAction::DataExport => "DataExport",
        AuditAction::SystemShutdown => "SystemShutdown",
    }
    .to_string()
}

fn action_from_str(s: &str) -> AuditAction {
    match s {
        "Logout" => AuditAction::Logout,
        "OrderSubmit" => AuditAction::OrderSubmit,
        "OrderCancel" => AuditAction::OrderCancel,
        "StrategyStart" => AuditAction::StrategyStart,
        "StrategyStop" => AuditAction::StrategyStop,
        "ConfigChange" => AuditAction::ConfigChange,
        "ApiKeyAccess" => AuditAction::ApiKeyAccess,
        "DataExport" => AuditAction::DataExport,
        "SystemShutdown" => AuditAction::SystemShutdown,
        _ => AuditAction::Login,
    }
}

impl From<AuditLogRecord> for AuditLog {
    fn from(record: AuditLogRecord) -> Self {
        Self {
            id: record.id,
            timestamp: record.created_at,
            user_id: record.user_id.to_string(),
            username: record.username,
            action: action_from_str(&record.action),
            resource: record.resource,
            details: record.details.unwrap_or(serde_json::Value::Null),
            ip_address: record.ip_address,
            success: record.success,
            error_message: record.error_message,
        }
    }
}

/// 审计日志记录器
pub struct AuditLogger {
    repo: Option<Arc<dyn AuditRepository>>,
}

impl AuditLogger {
    /// Create a logger. When a repository is supplied, each log entry is
    /// persisted concurrently; a write failure is logged but never fails the
    /// caller (audit logging is best-effort).
    pub fn new(repo: Option<Arc<dyn AuditRepository>>) -> Self {
        Self { repo }
    }

    /// 记录审计日志
    #[allow(clippy::too_many_arguments)]
    pub async fn log(
        &self,
        user_id: &str,
        username: &str,
        action: AuditAction,
        resource: &str,
        details: serde_json::Value,
        ip_address: Option<String>,
        success: bool,
        error_message: Option<String>,
    ) -> Result<AuditLog> {
        let log = AuditLog {
            id: 0,
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            action: action.clone(),
            resource: resource.to_string(),
            details,
            ip_address,
            success,
            error_message,
        };

        info!(
            action = ?log.action,
            user = %log.username,
            resource = %log.resource,
            result = if log.success { "success" } else { "failed" },
            audit_id = %log.id,
            "audit event recorded"
        );

        // Persist to the repository (best-effort; failure is logged only).
        if let Some(repo) = self.repo.as_ref() {
            let new_log = NewAuditLog {
                user_id: user_id.parse::<i64>().unwrap_or(0),
                username: log.username.clone(),
                action: action_to_str(&log.action),
                resource: log.resource.clone(),
                details: Some(log.details.clone()),
                ip_address: log.ip_address.clone(),
                success: log.success,
                error_message: log.error_message.clone(),
            };
            match repo.insert(&new_log).await {
                Ok(id) => {
                    info!(audit_id = %id, "audit event persisted");
                }
                Err(e) => warn!("Failed to persist audit log: {}", e),
            }
        }

        Ok(log)
    }

    /// 查询审计日志（分页 + 按用户 / action 过滤）。
    pub async fn query_logs(
        &self,
        user_id: Option<i64>,
        username: Option<String>,
        action: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>> {
        let Some(repo) = self.repo.as_ref() else {
            return Ok(Vec::new());
        };

        let filter = AuditFilter {
            user_id,
            username,
            action,
            limit,
            offset,
        };
        let (records, _total) = repo
            .find_all(&filter)
            .await
            .map_err(|e| quant_common::Error::Database(e.to_string()))?;
        Ok(records.into_iter().map(AuditLog::from).collect())
    }

    /// 记录登录
    pub async fn log_login(
        &self,
        user_id: &str,
        username: &str,
        ip: Option<String>,
        success: bool,
    ) -> Result<AuditLog> {
        self.log(
            user_id,
            username,
            AuditAction::Login,
            "authentication",
            serde_json::json!({}),
            ip,
            success,
            None,
        )
        .await
    }

    /// 记录订单提交
    pub async fn log_order_submit(
        &self,
        user_id: &str,
        username: &str,
        order_id: &str,
        symbol: &str,
        side: &str,
        quantity: &str,
    ) -> Result<AuditLog> {
        self.log(
            user_id,
            username,
            AuditAction::OrderSubmit,
            order_id,
            serde_json::json!({
                "symbol": symbol,
                "side": side,
                "quantity": quantity,
            }),
            None,
            true,
            None,
        )
        .await
    }

    /// 记录配置变更
    pub async fn log_config_change(
        &self,
        user_id: &str,
        username: &str,
        config_key: &str,
        old_value: &str,
        new_value: &str,
    ) -> Result<AuditLog> {
        self.log(
            user_id,
            username,
            AuditAction::ConfigChange,
            config_key,
            serde_json::json!({
                "old_value": old_value,
                "new_value": new_value,
            }),
            None,
            true,
            None,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use parking_lot::Mutex;
    use quant_repository::RepoError;
    use std::result::Result;

    struct InMemoryAuditRepository {
        logs: Mutex<Vec<NewAuditLog>>,
    }

    impl InMemoryAuditRepository {
        fn new() -> Self {
            Self {
                logs: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl AuditRepository for InMemoryAuditRepository {
        async fn insert(&self, row: &NewAuditLog) -> Result<i64, RepoError> {
            let mut logs = self.logs.lock();
            logs.push(row.clone());
            Ok(logs.len() as i64)
        }

        async fn find_all(
            &self,
            filter: &AuditFilter,
        ) -> Result<(Vec<AuditLogRecord>, i64), RepoError> {
            let logs = self.logs.lock();
            let mut records: Vec<AuditLogRecord> = logs
                .iter()
                .filter(|row| {
                    filter.user_id.map_or(true, |u| row.user_id == u)
                        && filter
                            .username
                            .as_ref()
                            .map_or(true, |name| &row.username == name)
                        && filter
                            .action
                            .as_ref()
                            .map_or(true, |action| &row.action == action)
                })
                .enumerate()
                .map(|(idx, row)| AuditLogRecord {
                    id: idx as i64 + 1,
                    user_id: row.user_id,
                    username: row.username.clone(),
                    action: row.action.clone(),
                    resource: row.resource.clone(),
                    details: row.details.clone(),
                    ip_address: row.ip_address.clone(),
                    success: row.success,
                    error_message: row.error_message.clone(),
                    created_at: Utc::now(),
                })
                .collect();
            let total = records.len() as i64;
            records = records
                .into_iter()
                .rev()
                .skip(filter.offset as usize)
                .take(filter.limit as usize)
                .collect();
            Ok((records, total))
        }
    }

    #[tokio::test]
    async fn test_audit_logger_persists_to_repository() {
        let repo = Arc::new(InMemoryAuditRepository::new());
        let logger = AuditLogger::new(Some(repo.clone()));
        let log = logger
            .log_login("1", "admin", Some("127.0.0.1".into()), true)
            .await
            .unwrap();
        assert_eq!(log.action, AuditAction::Login);
        assert_eq!(log.username, "admin");
        assert_eq!(repo.logs.lock().len(), 1);
    }

    #[tokio::test]
    async fn test_audit_logger_no_repo_still_logs() {
        let logger = AuditLogger::new(None);
        let log = logger
            .log_config_change("1", "admin", "risk.max_daily_loss", "100", "200")
            .await
            .unwrap();
        assert_eq!(log.action, AuditAction::ConfigChange);
        assert_eq!(log.resource, "risk.max_daily_loss");
    }

    #[tokio::test]
    async fn test_query_logs_with_action_filter() {
        let repo = Arc::new(InMemoryAuditRepository::new());
        let logger = AuditLogger::new(Some(repo.clone()));
        logger
            .log_order_submit("1", "admin", "ORD-1", "BTC-USDT", "buy", "0.1")
            .await
            .unwrap();
        logger.log_login("1", "admin", None, true).await.unwrap();

        let logs = logger
            .query_logs(None, None, Some("OrderSubmit".to_string()), 100, 0)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, AuditAction::OrderSubmit);
    }
}
