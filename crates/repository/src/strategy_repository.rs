use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_domain::types::StrategyParams;
use quant_domain::types::StrategyStatus;
use quant_domain::types::StrategyType;
use rust_decimal::Decimal;
use serde_json;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, instrument};

use crate::error::RepoError;

/// Database row type — maps 1:1 to `strategies` table columns.
#[derive(Debug, Clone, sqlx::FromRow)]
struct StrategyRow {
    #[allow(dead_code)]
    id: i32,
    strategy_id: String,
    strategy_name: String,
    strategy_type: String,
    params: serde_json::Value,
    enabled: bool,
    max_position: Decimal,
    max_daily_loss: Decimal,
    status: String,
    description: Option<String>,
    tags: serde_json::Value,
    symbols: serde_json::Value,
    instance_label: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    pub user_id: Option<i64>,
    pub version: i64,
}

impl StrategyRow {
    fn to_domain(&self) -> Result<StrategyParams, RepoError> {
        let strategy_type: StrategyType =
            serde_json::from_value(serde_json::Value::String(self.strategy_type.clone()))
                .map_err(|e| {
                    RepoError::Database(format!("invalid strategy_type: {}", e))
                })?;
        let status: StrategyStatus =
            serde_json::from_value(serde_json::Value::String(self.status.clone()))
                .map_err(|e| {
                    RepoError::Database(format!("invalid status: {}", e))
                })?;
        let tags: Vec<String> = serde_json::from_value(self.tags.clone())
            .unwrap_or_default();
        let symbols: Vec<String> = serde_json::from_value(self.symbols.clone())
            .unwrap_or_default();

        Ok(StrategyParams {
            strategy_id: self.strategy_id.clone(),
            strategy_name: self.strategy_name.clone(),
            strategy_type,
            params: self.params.clone(),
            enabled: self.enabled,
            max_position: self.max_position,
            max_daily_loss: self.max_daily_loss,
            status,
            description: self.description.clone(),
            tags,
            symbols,
            instance_label: self.instance_label.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            user_id: self.user_id.unwrap_or(0),
            version: self.version,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StrategySummaryRow {
    pub id: i32,
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: String,
    pub params: serde_json::Value,
    pub enabled: bool,
    pub status: String,
    pub max_position: Decimal,
    pub max_daily_loss: Decimal,
    pub description: Option<String>,
    pub tags: serde_json::Value,
    pub symbols: serde_json::Value,
    pub instance_label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: Option<i64>,
    pub version: i64,
}

impl StrategySummaryRow {
    pub fn to_domain(&self) -> Result<StrategyParams, RepoError> {
        let strategy_type: StrategyType =
            serde_json::from_value(serde_json::Value::String(self.strategy_type.clone()))
                .map_err(|e| RepoError::Database(format!("invalid strategy_type: {}", e)))?;
        let status: StrategyStatus =
            serde_json::from_value(serde_json::Value::String(self.status.clone()))
                .map_err(|e| RepoError::Database(format!("invalid status: {}", e)))?;
        let tags: Vec<String> = serde_json::from_value(self.tags.clone()).unwrap_or_default();
        let symbols: Vec<String> = serde_json::from_value(self.symbols.clone()).unwrap_or_default();

        Ok(StrategyParams {
            strategy_id: self.strategy_id.clone(),
            strategy_name: self.strategy_name.clone(),
            strategy_type,
            params: self.params.clone(),
            enabled: self.enabled,
            max_position: self.max_position,
            max_daily_loss: self.max_daily_loss,
            status,
            description: self.description.clone(),
            tags,
            symbols,
            instance_label: self.instance_label.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            user_id: self.user_id.unwrap_or(0),
            version: self.version,
        })
    }
}

/// Strategy data access trait.
#[async_trait]
pub trait StrategyRepository: Send + Sync + 'static {
    /// List strategies with optional filters + pagination (created_at DESC).
    async fn find_all(
        &self,
        search: Option<&str>,
        strategy_type: Option<StrategyType>,
        status: Option<StrategyStatus>,
        enabled: Option<bool>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<StrategySummaryRow>, i64), RepoError>;

    /// Count strategies matching the optional filters.
    async fn count(
        &self,
        search: Option<&str>,
        strategy_type: Option<StrategyType>,
        status: Option<StrategyStatus>,
        enabled: Option<bool>,
    ) -> Result<i64, RepoError>;

    /// Find a single strategy by its stable `strategy_id`.
    async fn find_by_id(&self, strategy_id: &str) -> Result<Option<StrategyParams>, RepoError>;

    /// Insert a new strategy. Returns the auto-generated `id`.
    async fn insert(&self, params: &StrategyParams) -> Result<i32, RepoError>;

    /// Update an existing strategy (identified by `strategy_id`). Returns `true` if updated.
    async fn update(&self, params: &StrategyParams) -> Result<bool, RepoError>;

    /// Atomically update a strategy only if its current version matches `expected_version`.
    ///
    /// Returns `Ok(true)` when exactly one row was updated (version matched).
    /// Returns `Ok(false)` when the precondition did not hold (zero rows affected — a conflict).
    async fn update_with_version(
        &self,
        strategy_id: &str,
        params: &StrategyParams,
        expected_version: i64,
    ) -> Result<bool, RepoError>;

    /// Delete a strategy by `strategy_id`. Returns `true` if deleted.
    async fn delete_by_id(&self, strategy_id: &str) -> Result<bool, RepoError>;

    /// Update only the status of a strategy. Returns `true` if updated.
    async fn update_status(
        &self,
        strategy_id: &str,
        status: StrategyStatus,
        updated_by: Option<&str>,
    ) -> Result<bool, RepoError>;

    /// Atomically update status only if current status matches `expected_old_status`.
    ///
    /// SQL: `UPDATE strategies SET status = $new, updated_at = NOW()
    ///      WHERE strategy_id = $id AND status = $expected`
    ///
    /// Returns `true` when exactly one row was updated (status matched the
    /// precondition); returns `false` when the precondition did not hold
    /// (zero rows affected). On real DBs this is the compare-and-set
    /// primitive that closes the TOCTOU window in lifecycle operations.
    async fn update_status_if(
        &self,
        strategy_id: &str,
        new_status: StrategyStatus,
        expected_old_status: StrategyStatus,
        updated_by: Option<&str>,
    ) -> Result<bool, RepoError>;

    /// Get aggregated statistics about strategies.
    async fn stats(&self) -> Result<StrategyStats, RepoError>;
}

/// Aggregated statistics about strategies.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StrategyStats {
    pub total: i64,
    pub enabled: i64,
    pub disabled: i64,
    pub draft: i64,
    pub backtesting: i64,
    pub deployed: i64,
    pub running: i64,
    pub paused: i64,
    pub archived: i64,
    pub trend_following: i64,
    pub mean_reversion: i64,
    pub arbitrage: i64,
    pub market_making: i64,
    pub statistical: i64,
    pub machine_learning: i64,
    pub custom: i64,
}

/// PostgreSQL implementation of `StrategyRepository`.
#[derive(Debug, Clone)]
pub struct PgStrategyRepository {
    pool: Arc<PgPool>,
}

impl PgStrategyRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StrategyRepository for PgStrategyRepository {
    #[instrument(skip(self), fields(limit, offset))]
    async fn find_all(
        &self,
        search: Option<&str>,
        strategy_type: Option<StrategyType>,
        status: Option<StrategyStatus>,
        enabled: Option<bool>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<StrategySummaryRow>, i64), RepoError> {
        let search_pattern = search.map(|s| format!("%{}%", s));
        let st_str = strategy_type.as_ref().map(|st| format!("{:?}", st));
        let status_str = status.as_ref().map(|s| format!("{:?}", s));

        let mut query = sqlx::QueryBuilder::new(
            "SELECT id, strategy_id, strategy_name, strategy_type, params, enabled, status, \
              max_position, max_daily_loss, description, tags, symbols, instance_label, created_at, updated_at, \
              user_id, version FROM strategies WHERE 1=1"
        );

        if let Some(ref pattern) = search_pattern {
            query.push(" AND (strategy_id ILIKE ");
            query.push_bind(pattern);
            query.push(" OR strategy_name ILIKE ");
            query.push_bind(pattern);
            query.push(")");
        }

        if let Some(ref st) = st_str {
            query.push(" AND strategy_type = ");
            query.push_bind(st);
        }

        if let Some(ref s) = status_str {
            query.push(" AND status = ");
            query.push_bind(s);
        }

        if let Some(e) = enabled {
            query.push(" AND enabled = ");
            query.push_bind(e);
        }

        query.push(" ORDER BY created_at DESC LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows: Vec<StrategySummaryRow> = query
            .build_query_as()
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to query strategies: {}", e);
                RepoError::from(e)
            })?;

        let total = self.count(search, strategy_type, status, enabled).await?;
        Ok((rows, total))
    }

    #[instrument(skip(self))]
    async fn count(
        &self,
        search: Option<&str>,
        strategy_type: Option<StrategyType>,
        status: Option<StrategyStatus>,
        enabled: Option<bool>,
    ) -> Result<i64, RepoError> {
        let search_pattern = search.map(|s| format!("%{}%", s));
        let st_str = strategy_type.map(|st| format!("{:?}", st));
        let status_str = status.map(|s| format!("{:?}", s));

        let mut query = sqlx::QueryBuilder::new("SELECT COUNT(*) FROM strategies WHERE 1=1");

        if let Some(ref pattern) = search_pattern {
            query.push(" AND (strategy_id ILIKE ");
            query.push_bind(pattern);
            query.push(" OR strategy_name ILIKE ");
            query.push_bind(pattern);
            query.push(")");
        }

        if let Some(ref st) = st_str {
            query.push(" AND strategy_type = ");
            query.push_bind(st);
        }

        if let Some(ref s) = status_str {
            query.push(" AND status = ");
            query.push_bind(s);
        }

        if let Some(e) = enabled {
            query.push(" AND enabled = ");
            query.push_bind(e);
        }

        let count: (i64,) = query
            .build_query_as()
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to count strategies: {}", e);
                RepoError::from(e)
            })?;

        Ok(count.0)
    }

    #[instrument(skip(self), fields(%strategy_id))]
    async fn find_by_id(&self, strategy_id: &str) -> Result<Option<StrategyParams>, RepoError> {
        let row = sqlx::query_as::<_, StrategyRow>(
            r#"
            SELECT id, strategy_id, strategy_name, strategy_type,
                   params, enabled,
                   max_position, max_daily_loss,
                   status, description, tags, symbols,
                   instance_label,
                   created_at, updated_at,
                   user_id, version
            FROM strategies
            WHERE strategy_id = $1
            "#,
        )
        .bind(strategy_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query strategy {}: {}", strategy_id, e);
            RepoError::from(e)
        })?;

        row.map(|r| r.to_domain()).transpose()
    }

    #[instrument(skip(self, params), fields(strategy_id = %params.strategy_id))]
    async fn insert(&self, params: &StrategyParams) -> Result<i32, RepoError> {
        let status_str = format!("{:?}", params.status);
        let strategy_type_str = format!("{:?}", params.strategy_type);
        let tags_json = serde_json::to_value(&params.tags)
            .map_err(|e| RepoError::Database(format!("serialize tags: {}", e)))?;
        let symbols_json = serde_json::to_value(&params.symbols)
            .map_err(|e| RepoError::Database(format!("serialize symbols: {}", e)))?;

        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO strategies (
                strategy_id, strategy_name, strategy_type,
                params, enabled,
                max_position, max_daily_loss,
                status, description, tags, symbols,
                instance_label,
                created_at, updated_at, version
            ) VALUES (
                $1, $2, $3,
                $4, $5,
                $6, $7,
                $8, $9, $10, $11,
                $12,
                $13, $14, $15
            )
            RETURNING id
            "#,
        )
        .bind(&params.strategy_id)
        .bind(&params.strategy_name)
        .bind(&strategy_type_str)
        .bind(&params.params)
        .bind(params.enabled)
        .bind(params.max_position)
        .bind(params.max_daily_loss)
        .bind(&status_str)
        .bind(&params.description)
        .bind(&tags_json)
        .bind(&symbols_json)
        .bind(&params.instance_label)
        .bind(params.created_at)
        .bind(params.updated_at)
        .bind(params.version)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to insert strategy {}: {}", params.strategy_id, e);
            RepoError::from(e)
        })?;

        Ok(row.0)
    }

    #[instrument(skip(self, params), fields(strategy_id = %params.strategy_id))]
    async fn update(&self, params: &StrategyParams) -> Result<bool, RepoError> {
        let status_str = format!("{:?}", params.status);
        let strategy_type_str = format!("{:?}", params.strategy_type);
        let tags_json = serde_json::to_value(&params.tags)
            .map_err(|e| RepoError::Database(format!("serialize tags: {}", e)))?;
        let symbols_json = serde_json::to_value(&params.symbols)
            .map_err(|e| RepoError::Database(format!("serialize symbols: {}", e)))?;

        let affected = sqlx::query(
            r#"
            UPDATE strategies SET
                strategy_name = $1,
                strategy_type = $2,
                params = $3,
                enabled = $4,
                max_position = $5,
                max_daily_loss = $6,
                status = $7,
                description = $8,
                tags = $9,
                symbols = $10,
                instance_label = $11,
                updated_at = $12,
                version = version + 1
            WHERE strategy_id = $13
            "#,
        )
        .bind(&params.strategy_name)
        .bind(&strategy_type_str)
        .bind(&params.params)
        .bind(params.enabled)
        .bind(params.max_position)
        .bind(params.max_daily_loss)
        .bind(&status_str)
        .bind(&params.description)
        .bind(&tags_json)
        .bind(&symbols_json)
        .bind(&params.instance_label)
        .bind(params.updated_at)
        .bind(&params.strategy_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update strategy {}: {}", params.strategy_id, e);
            RepoError::from(e)
        })?;

        Ok(affected.rows_affected() > 0)
    }

    #[instrument(skip(self, params), fields(strategy_id = %strategy_id, expected_version))]
    async fn update_with_version(
        &self,
        strategy_id: &str,
        params: &StrategyParams,
        expected_version: i64,
    ) -> Result<bool, RepoError> {
        let status_str = format!("{:?}", params.status);
        let strategy_type_str = format!("{:?}", params.strategy_type);
        let tags_json = serde_json::to_value(&params.tags)
            .map_err(|e| RepoError::Database(format!("serialize tags: {}", e)))?;
        let symbols_json = serde_json::to_value(&params.symbols)
            .map_err(|e| RepoError::Database(format!("serialize symbols: {}", e)))?;

        let affected = sqlx::query(
            r#"
            UPDATE strategies SET
                strategy_name = $1,
                strategy_type = $2,
                params = $3,
                enabled = $4,
                max_position = $5,
                max_daily_loss = $6,
                status = $7,
                description = $8,
                tags = $9,
                symbols = $10,
                instance_label = $11,
                updated_at = $12,
                version = version + 1
            WHERE strategy_id = $13 AND version = $14
            "#,
        )
        .bind(&params.strategy_name)
        .bind(&strategy_type_str)
        .bind(&params.params)
        .bind(params.enabled)
        .bind(params.max_position)
        .bind(params.max_daily_loss)
        .bind(&status_str)
        .bind(&params.description)
        .bind(&tags_json)
        .bind(&symbols_json)
        .bind(&params.instance_label)
        .bind(params.updated_at)
        .bind(strategy_id)
        .bind(expected_version)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update strategy {} with version check: {}", strategy_id, e);
            RepoError::from(e)
        })?;

        Ok(affected.rows_affected() > 0)
    }

    #[instrument(skip(self), fields(%strategy_id))]
    async fn delete_by_id(&self, strategy_id: &str) -> Result<bool, RepoError> {
        let affected = sqlx::query("DELETE FROM strategies WHERE strategy_id = $1")
            .bind(strategy_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to delete strategy {}: {}", strategy_id, e);
                RepoError::from(e)
            })?;

        Ok(affected.rows_affected() > 0)
    }

    #[instrument(skip(self), fields(%strategy_id))]
    async fn update_status(
        &self,
        strategy_id: &str,
        status: StrategyStatus,
        _updated_by: Option<&str>,
    ) -> Result<bool, RepoError> {
        let status_str = format!("{:?}", status);
        let updated_at = Utc::now();

        let affected = sqlx::query(
            r#"
            UPDATE strategies SET
                status = $1,
                updated_at = $2
            WHERE strategy_id = $3
            "#,
        )
        .bind(&status_str)
        .bind(updated_at)
        .bind(strategy_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update status for strategy {}: {}", strategy_id, e);
            RepoError::from(e)
        })?;

        Ok(affected.rows_affected() > 0)
    }

    #[instrument(skip(self), fields(%strategy_id))]
    async fn update_status_if(
        &self,
        strategy_id: &str,
        new_status: StrategyStatus,
        expected_old_status: StrategyStatus,
        _updated_by: Option<&str>,
    ) -> Result<bool, RepoError> {
        let new_status_str = format!("{:?}", new_status);
        let expected_status_str = format!("{:?}", expected_old_status);
        let updated_at = Utc::now();

        let affected = sqlx::query(
            r#"
            UPDATE strategies SET
                status = $1,
                updated_at = $2
            WHERE strategy_id = $3 AND status = $4
            "#,
        )
        .bind(&new_status_str)
        .bind(updated_at)
        .bind(strategy_id)
        .bind(&expected_status_str)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!(
                "Failed to CAS-update status for strategy {}: {}",
                strategy_id, e
            );
            RepoError::from(e)
        })?;

        Ok(affected.rows_affected() > 0)
    }

    #[instrument(skip(self))]
    async fn stats(&self) -> Result<StrategyStats, RepoError> {
        let row: (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) =
            sqlx::query_as(
                r#"
                SELECT
                    COUNT(*) as total,
                    COUNT(*) FILTER (WHERE enabled = true) as enabled,
                    COUNT(*) FILTER (WHERE enabled = false) as disabled,
                    COUNT(*) FILTER (WHERE status = 'Draft') as draft,
                    COUNT(*) FILTER (WHERE status = 'Backtesting') as backtesting,
                    COUNT(*) FILTER (WHERE status = 'Deployed') as deployed,
                    COUNT(*) FILTER (WHERE status = 'Running') as running,
                    COUNT(*) FILTER (WHERE status = 'Paused') as paused,
                    COUNT(*) FILTER (WHERE status = 'Archived') as archived,
                    COUNT(*) FILTER (WHERE strategy_type = 'TrendFollowing') as trend_following,
                    COUNT(*) FILTER (WHERE strategy_type = 'MeanReversion') as mean_reversion,
                    COUNT(*) FILTER (WHERE strategy_type = 'Arbitrage') as arbitrage,
                    COUNT(*) FILTER (WHERE strategy_type = 'MarketMaking') as market_making,
                    COUNT(*) FILTER (WHERE strategy_type = 'Statistical') as statistical,
                    COUNT(*) FILTER (WHERE strategy_type = 'MachineLearning') as machine_learning,
                    COUNT(*) FILTER (WHERE strategy_type = 'Custom') as custom
                FROM strategies
                "#,
            )
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| {
                error!("Failed to get strategy stats: {}", e);
                RepoError::from(e)
            })?;

        Ok(StrategyStats {
            total: row.0,
            enabled: row.1,
            disabled: row.2,
            draft: row.3,
            backtesting: row.4,
            deployed: row.5,
            running: row.6,
            paused: row.7,
            archived: row.8,
            trend_following: row.9,
            mean_reversion: row.10,
            arbitrage: row.11,
            market_making: row.12,
            statistical: row.13,
            machine_learning: row.14,
            custom: row.15,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal_macros::dec;
    use serde_json::json;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn make_row() -> StrategyRow {
        StrategyRow {
            id: 42,
            strategy_id: "strat_test_001".into(),
            strategy_name: "Test Strategy".into(),
            strategy_type: "TrendFollowing".into(),
            params: json!({"period": 14, "threshold": 0.5}),
            enabled: true,
            max_position: dec!(10000),
            max_daily_loss: dec!(5000),
            status: "Draft".into(),
            description: Some("A test strategy".into()),
            tags: json!(["momentum", "trend"]),
            symbols: json!(["BTC/USDT", "ETH/USDT"]),
            instance_label: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id: Some(0),
        }
    }

    fn make_summary_row() -> StrategySummaryRow {
        StrategySummaryRow {
            id: 99,
            strategy_id: "strat_summary_001".into(),
            strategy_name: "Summary Strategy".into(),
            strategy_type: "MeanReversion".into(),
            params: json!({"lookback": 20}),
            enabled: false,
            max_position: dec!(5000),
            max_daily_loss: dec!(2500),
            status: "Running".into(),
            description: Some("Summary description".into()),
            tags: json!(["mean", "reversion"]),
            symbols: json!(["SOL/USDT"]),
            instance_label: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_id: Some(0),
        }
    }

    // ── StrategyRow::to_domain ─────────────────────────────────────────────────

    #[test]
    fn test_strategy_row_to_domain_valid() {
        let row = make_row();
        let result = row.to_domain();
        assert!(result.is_ok());
        let domain = result.unwrap();

        assert_eq!(domain.strategy_id, "strat_test_001");
        assert_eq!(domain.strategy_name, "Test Strategy");
        assert_eq!(domain.strategy_type, StrategyType::TrendFollowing);
        assert_eq!(domain.params, json!({"period": 14, "threshold": 0.5}));
        assert!(domain.enabled);
        assert_eq!(domain.max_position, dec!(10000));
        assert_eq!(domain.max_daily_loss, dec!(5000));
        assert_eq!(domain.status, StrategyStatus::Draft);
        assert_eq!(domain.description, Some("A test strategy".into()));
        assert_eq!(domain.tags, vec!["momentum", "trend"]);
        assert_eq!(domain.symbols, vec!["BTC/USDT", "ETH/USDT"]);
    }

    #[test]
    fn test_strategy_row_to_domain_invalid_strategy_type() {
        let mut row = make_row();
        row.strategy_type = "InvalidType".into();
        let result = row.to_domain();
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_row_to_domain_invalid_status() {
        let mut row = make_row();
        row.status = "InvalidStatus".into();
        let result = row.to_domain();
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_row_to_domain_null_tags() {
        let mut row = make_row();
        row.tags = json!(null);
        let domain = row.to_domain().unwrap();
        assert!(domain.tags.is_empty());
    }

    #[test]
    fn test_strategy_row_to_domain_null_symbols() {
        let mut row = make_row();
        row.symbols = json!(null);
        let domain = row.to_domain().unwrap();
        assert!(domain.symbols.is_empty());
    }

    #[test]
    fn test_strategy_row_to_domain_empty_tags() {
        let mut row = make_row();
        row.tags = json!([]);
        let domain = row.to_domain().unwrap();
        assert!(domain.tags.is_empty());
    }

    #[test]
    fn test_strategy_row_to_domain_empty_symbols() {
        let mut row = make_row();
        row.symbols = json!([]);
        let domain = row.to_domain().unwrap();
        assert!(domain.symbols.is_empty());
    }

    #[test]
    fn test_strategy_row_to_domain_tags_non_array() {
        let mut row = make_row();
        row.tags = json!("not_an_array");
        let domain = row.to_domain().unwrap();
        assert!(domain.tags.is_empty());
    }

    #[test]
    fn test_strategy_row_to_domain_symbols_non_array() {
        let mut row = make_row();
        row.symbols = json!("not_an_array");
        let domain = row.to_domain().unwrap();
        assert!(domain.symbols.is_empty());
    }

    // ── StrategySummaryRow::to_domain ─────────────────────────────────────────

    #[test]
    fn test_strategy_summary_row_to_domain_valid() {
        let row = make_summary_row();
        let result = row.to_domain();
        assert!(result.is_ok());
        let domain = result.unwrap();

        assert_eq!(domain.strategy_id, "strat_summary_001");
        assert_eq!(domain.strategy_name, "Summary Strategy");
        assert_eq!(domain.strategy_type, StrategyType::MeanReversion);
        assert_eq!(domain.params, json!({"lookback": 20}));
        assert!(!domain.enabled);
        assert_eq!(domain.max_position, dec!(5000));
        assert_eq!(domain.max_daily_loss, dec!(2500));
        assert_eq!(domain.status, StrategyStatus::Running);
        assert_eq!(domain.description, Some("Summary description".into()));
        assert_eq!(domain.tags, vec!["mean", "reversion"]);
        assert_eq!(domain.symbols, vec!["SOL/USDT"]);
    }

    #[test]
    fn test_strategy_summary_row_to_domain_invalid_strategy_type() {
        let mut row = make_summary_row();
        row.strategy_type = "BogusType".into();
        let result = row.to_domain();
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_summary_row_to_domain_invalid_status() {
        let mut row = make_summary_row();
        row.status = "BogusStatus".into();
        let result = row.to_domain();
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_summary_row_to_domain_null_tags() {
        let mut row = make_summary_row();
        row.tags = json!(null);
        let domain = row.to_domain().unwrap();
        assert!(domain.tags.is_empty());
    }

    #[test]
    fn test_strategy_summary_row_to_domain_null_symbols() {
        let mut row = make_summary_row();
        row.symbols = json!(null);
        let domain = row.to_domain().unwrap();
        assert!(domain.symbols.is_empty());
    }

    #[test]
    fn test_strategy_summary_row_to_domain_tags_non_array() {
        let mut row = make_summary_row();
        row.tags = json!("not_an_array");
        let domain = row.to_domain().unwrap();
        assert!(domain.tags.is_empty());
    }

    #[test]
    fn test_strategy_summary_row_to_domain_symbols_non_array() {
        let mut row = make_summary_row();
        row.symbols = json!("not_an_array");
        let domain = row.to_domain().unwrap();
        assert!(domain.symbols.is_empty());
    }
}
