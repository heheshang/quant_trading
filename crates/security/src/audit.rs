use chrono::{DateTime, Utc};
use quant_common::Result;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

/// 审计日志类型
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub id: Uuid,
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

/// 审计日志记录器
pub struct AuditLogger;

impl AuditLogger {
    /// 记录审计日志
    #[allow(clippy::too_many_arguments)]
    pub async fn log(
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
            id: Uuid::new_v4(),
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

        // 记录到日志系统
        info!(
            audit_log = ?log,
            "Audit: {} by {} on {} - {}",
            format!("{:?}", log.action),
            log.username,
            log.resource,
            if log.success { "SUCCESS" } else { "FAILED" }
        );

        // TODO: 存储到数据库
        // INSERT INTO audit_logs ...

        Ok(log)
    }

    /// 记录登录
    pub async fn log_login(
        user_id: &str,
        username: &str,
        ip: Option<String>,
        success: bool,
    ) -> Result<AuditLog> {
        Self::log(
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
        user_id: &str,
        username: &str,
        order_id: &str,
        symbol: &str,
        side: &str,
        quantity: &str,
    ) -> Result<AuditLog> {
        Self::log(
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
        user_id: &str,
        username: &str,
        config_key: &str,
        old_value: &str,
        new_value: &str,
    ) -> Result<AuditLog> {
        Self::log(
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

    #[tokio::test]
    async fn test_audit_logging() {
        let log =
            AuditLogger::log_login("user123", "testuser", Some("127.0.0.1".to_string()), true)
                .await
                .unwrap();

        assert_eq!(log.user_id, "user123");
        assert_eq!(log.username, "testuser");
        assert!(log.success);
    }
}
