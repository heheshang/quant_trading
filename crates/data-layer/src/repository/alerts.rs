use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_common::types::{Alert, AlertLevel};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, instrument};

use crate::repository::error::RepoError;

/// Database row type — maps 1:1 to the `alerts` table columns.
#[derive(Debug, Clone, sqlx::FromRow)]
struct AlertRow {
    id: i64,
    level: String,
    source: String,
    message: String,
    acknowledged: bool,
    created_at: DateTime<Utc>,
}

fn level_to_str(level: &AlertLevel) -> String {
    match level {
        AlertLevel::Info => "Info".to_string(),
        AlertLevel::Warning => "Warning".to_string(),
        AlertLevel::Critical => "Critical".to_string(),
    }
}

fn level_from_str(s: &str) -> Option<AlertLevel> {
    match s {
        "Info" => Some(AlertLevel::Info),
        "Warning" => Some(AlertLevel::Warning),
        "Critical" => Some(AlertLevel::Critical),
        _ => None,
    }
}

impl AlertRow {
    fn to_alert(&self) -> Alert {
        Alert {
            alert_id: self.id,
            level: level_from_str(&self.level).unwrap_or(AlertLevel::Info),
            source: self.source.clone(),
            message: self.message.clone(),
            timestamp: self.created_at,
            acknowledged: self.acknowledged,
        }
    }
}

/// Alert data access trait.
#[async_trait]
pub trait AlertRepository: Send + Sync + 'static {
    /// Insert an alert. Returns the stored alert (with the DB-assigned id).
    async fn insert(&self, alert: &Alert) -> Result<Alert, RepoError>;

    /// List alerts (created_at DESC), paginated.
    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Alert>, RepoError>;

    /// List alerts filtered by level (created_at DESC), paginated.
    async fn find_by_level(
        &self,
        level: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Alert>, RepoError>;

    /// List alerts filtered by source (created_at DESC), paginated.
    async fn find_by_source(
        &self,
        source: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Alert>, RepoError>;

    /// List alerts within a time range (created_at DESC), paginated.
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Alert>, RepoError>;

    /// Mark an alert as acknowledged. Returns `true` if a row was updated.
    async fn acknowledge(&self, alert_id: i64) -> Result<bool, RepoError>;

    /// Count all alerts.
    async fn count(&self) -> Result<i64, RepoError>;
}

/// PostgreSQL implementation of [`AlertRepository`].
#[derive(Debug, Clone)]
pub struct PgAlertRepository {
    pool: Arc<PgPool>,
}

impl PgAlertRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AlertRepository for PgAlertRepository {
    #[instrument(skip(self, alert), fields(level = ?alert.level, source = %alert.source))]
    async fn insert(&self, alert: &Alert) -> Result<Alert, RepoError> {
        let level = level_to_str(&alert.level);
        let row = sqlx::query_as::<_, AlertRow>(
            r#"
            INSERT INTO alerts (level, source, message, acknowledged, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, level, source, message, acknowledged, created_at
            "#,
        )
        .bind(level)
        .bind(&alert.source)
        .bind(&alert.message)
        .bind(alert.acknowledged)
        .bind(alert.timestamp)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to insert alert: {}", e);
            RepoError::from(e)
        })?;

        Ok(row.to_alert())
    }

    #[instrument(skip(self), fields(limit, offset))]
    async fn find_all(&self, limit: i64, offset: i64) -> Result<Vec<Alert>, RepoError> {
        let rows = sqlx::query_as::<_, AlertRow>(
            r#"
            SELECT id, level, source, message, acknowledged, created_at
            FROM alerts
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query alerts: {}", e);
            RepoError::from(e)
        })?;

        Ok(rows.into_iter().map(|r| r.to_alert()).collect())
    }

    #[instrument(skip(self), fields(level, limit, offset))]
    async fn find_by_level(
        &self,
        level: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Alert>, RepoError> {
        let rows = sqlx::query_as::<_, AlertRow>(
            r#"
            SELECT id, level, source, message, acknowledged, created_at
            FROM alerts
            WHERE level = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(level)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query alerts by level: {}", e);
            RepoError::from(e)
        })?;

        Ok(rows.into_iter().map(|r| r.to_alert()).collect())
    }

    #[instrument(skip(self), fields(source, limit, offset))]
    async fn find_by_source(
        &self,
        source: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Alert>, RepoError> {
        let rows = sqlx::query_as::<_, AlertRow>(
            r#"
            SELECT id, level, source, message, acknowledged, created_at
            FROM alerts
            WHERE source = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(source)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query alerts by source: {}", e);
            RepoError::from(e)
        })?;

        Ok(rows.into_iter().map(|r| r.to_alert()).collect())
    }

    #[instrument(skip(self), fields(start, end, limit, offset))]
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Alert>, RepoError> {
        let rows = sqlx::query_as::<_, AlertRow>(
            r#"
            SELECT id, level, source, message, acknowledged, created_at
            FROM alerts
            WHERE created_at >= $1 AND created_at <= $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query alerts by time range: {}", e);
            RepoError::from(e)
        })?;

        Ok(rows.into_iter().map(|r| r.to_alert()).collect())
    }

    #[instrument(skip(self), fields(alert_id))]
    async fn acknowledge(&self, alert_id: i64) -> Result<bool, RepoError> {
        let result = sqlx::query(
            r#"
            UPDATE alerts
            SET acknowledged = TRUE, acknowledged_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(alert_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to acknowledge alert {}: {}", alert_id, e);
            RepoError::from(e)
        })?;

        Ok(result.rows_affected() > 0)
    }

    #[instrument(skip(self))]
    async fn count(&self) -> Result<i64, RepoError> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM alerts")
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to count alerts: {}", e);
                RepoError::from(e)
            })?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_string_roundtrip() {
        assert_eq!(level_to_str(&AlertLevel::Info), "Info");
        assert_eq!(level_to_str(&AlertLevel::Warning), "Warning");
        assert_eq!(level_to_str(&AlertLevel::Critical), "Critical");
        assert_eq!(level_from_str("Warning"), Some(AlertLevel::Warning));
        assert_eq!(level_from_str("unknown"), None);
    }

    #[test]
    fn test_alert_row_to_domain() {
        let row = AlertRow {
            id: 42,
            level: "Critical".to_string(),
            source: "Risk".to_string(),
            message: "margin breach".to_string(),
            acknowledged: false,
            created_at: Utc::now(),
        };
        let alert = row.to_alert();
        assert_eq!(alert.alert_id, 42);
        assert_eq!(alert.level, AlertLevel::Critical);
        assert_eq!(alert.message, "margin breach");
        assert!(!alert.acknowledged);
    }
}
