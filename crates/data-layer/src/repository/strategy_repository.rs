use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_domain::types::StrategyParams;
use quant_domain::types::StrategyStatus;
use quant_domain::types::StrategyType;
use rust_decimal::Decimal;
use serde_json;
use sqlx::PgPool;
use std::sync::Arc;

use crate::repository::error::RepoError;

mod pg_impl;

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
            version: 0,
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
            version: 0,
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
