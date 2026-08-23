//! `StrategyRepository` trait 的 PostgreSQL 实现。

use chrono::Utc;
use quant_domain::types::{StrategyParams, StrategyStatus, StrategyType};
use serde_json;
use tracing::{error, instrument};

use crate::error::RepoError;

use super::{
    PgStrategyRepository, StrategyRepository, StrategyRow, StrategyStats, StrategySummaryRow,
};

#[async_trait::async_trait]
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
            error!(
                "Failed to update strategy {} with version check: {}",
                strategy_id, e
            );
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
            error!(
                "Failed to update status for strategy {}: {}",
                strategy_id, e
            );
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
        let row: (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
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
                    COUNT(*) FILTER (WHERE strategy_type = 'MeanReversion') as mean_reversion
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
        })
    }
}
