use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, instrument};

use crate::repository::error::RepoError;

/// Database row type — maps 1:1 to the `audit_logs` table columns.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AuditLogRecord {
    pub id: i64,
    pub user_id: Option<i64>,
    pub username: String,
    pub action: String,
    pub resource: String,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Insert payload for [`AuditRepository::insert`].
#[derive(Debug, Clone)]
pub struct NewAuditLog {
    pub user_id: Option<i64>,
    pub username: String,
    pub action: String,
    pub resource: String,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Filter for [`AuditRepository::find_all`].
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub action: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Audit log data access trait.
#[async_trait]
pub trait AuditRepository: Send + Sync + 'static {
    /// Insert an audit log. Returns the DB-assigned id.
    async fn insert(&self, row: &NewAuditLog) -> Result<i64, RepoError>;

    /// Query audit logs with optional filters, paginated.
    /// Returns the matching rows and the total count (before pagination).
    async fn find_all(&self, filter: &AuditFilter)
        -> Result<(Vec<AuditLogRecord>, i64), RepoError>;
}

/// PostgreSQL implementation of [`AuditRepository`].
#[derive(Debug, Clone)]
pub struct PgAuditRepository {
    pool: Arc<PgPool>,
}

impl PgAuditRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditRepository for PgAuditRepository {
    #[instrument(skip(self, row), fields(user_id = ?row.user_id, action = %row.action))]
    async fn insert(&self, row: &NewAuditLog) -> Result<i64, RepoError> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO audit_logs (
                user_id, username, action, resource, details,
                ip_address, success, error_message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id
            "#,
        )
        .bind(row.user_id)
        .bind(&row.username)
        .bind(&row.action)
        .bind(&row.resource)
        .bind(&row.details)
        .bind(&row.ip_address)
        .bind(row.success)
        .bind(&row.error_message)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to insert audit log: {}", e);
            RepoError::from(e)
        })?;

        Ok(id)
    }

    #[instrument(skip(self, filter), fields(limit = filter.limit, offset = filter.offset))]
    async fn find_all(
        &self,
        filter: &AuditFilter,
    ) -> Result<(Vec<AuditLogRecord>, i64), RepoError> {
        let limit = filter.limit;
        let offset = filter.offset;

        let count_row = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM audit_logs
            WHERE ($1::bigint IS NULL OR user_id = $1)
              AND ($2::text IS NULL OR username = $2)
              AND ($3::text IS NULL OR action = $3)
            "#,
        )
        .bind(filter.user_id)
        .bind(&filter.username)
        .bind(&filter.action)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to count audit logs: {}", e);
            RepoError::from(e)
        })?;

        let rows = sqlx::query_as::<_, AuditLogRecord>(
            r#"
            SELECT id, user_id, username, action, resource, details,
                   ip_address, success, error_message, created_at
            FROM audit_logs
            WHERE ($1::bigint IS NULL OR user_id = $1)
              AND ($2::text IS NULL OR username = $2)
              AND ($3::text IS NULL OR action = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(filter.user_id)
        .bind(&filter.username)
        .bind(&filter.action)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query audit logs: {}", e);
            RepoError::from(e)
        })?;

        Ok((rows, count_row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_filter_default() {
        let filter = AuditFilter::default();
        assert_eq!(filter.user_id, None);
        assert_eq!(filter.limit, 0);
    }

    #[test]
    fn test_new_audit_log_roundtrip() {
        let row = NewAuditLog {
            user_id: Some(1),
            username: "admin".to_string(),
            action: "Login".to_string(),
            resource: "auth".to_string(),
            details: Some(serde_json::json!({"k": "v"})),
            ip_address: None,
            success: true,
            error_message: None,
        };
        assert_eq!(row.username, "admin");
        assert_eq!(row.action, "Login");
    }
}
