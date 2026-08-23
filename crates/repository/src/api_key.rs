use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, instrument};

use crate::error::RepoError;

/// Database row type — maps 1:1 to the `api_keys` table columns.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub user_id: i64,
    pub exchange: String,
    pub api_key: String,
    pub encrypted_secret: String,
    pub passphrase: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Insert payload for [`ApiKeyRepository::insert`].
#[derive(Debug, Clone)]
pub struct NewApiKey {
    pub user_id: i64,
    pub exchange: String,
    pub api_key: String,
    pub encrypted_secret: String,
    pub passphrase: Option<String>,
}

/// API key data access trait.
#[async_trait]
pub trait ApiKeyRepository: Send + Sync + 'static {
    /// Insert an API key. Returns the DB-assigned id.
    async fn insert(&self, row: &NewApiKey) -> Result<i64, RepoError>;

    /// List API keys for a user (created_at DESC).
    async fn find_all(&self, user_id: i64) -> Result<Vec<ApiKeyRecord>, RepoError>;

    /// List API keys for a user and exchange.
    async fn find_by_exchange(
        &self,
        user_id: i64,
        exchange: &str,
    ) -> Result<Vec<ApiKeyRecord>, RepoError>;

    /// Toggle an API key's active flag. Returns `true` if a row was updated.
    async fn set_active(&self, id: i64, active: bool) -> Result<bool, RepoError>;
}

/// PostgreSQL implementation of [`ApiKeyRepository`].
#[derive(Debug, Clone)]
pub struct PgApiKeyRepository {
    pool: Arc<PgPool>,
}

impl PgApiKeyRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApiKeyRepository for PgApiKeyRepository {
    #[instrument(skip(self, row), fields(user_id = %row.user_id, exchange = %row.exchange))]
    async fn insert(&self, row: &NewApiKey) -> Result<i64, RepoError> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO api_keys (user_id, exchange, api_key, encrypted_secret, passphrase)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(row.user_id)
        .bind(&row.exchange)
        .bind(&row.api_key)
        .bind(&row.encrypted_secret)
        .bind(&row.passphrase)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to insert api key: {}", e);
            RepoError::from(e)
        })?;

        Ok(id)
    }

    #[instrument(skip(self), fields(user_id))]
    async fn find_all(&self, user_id: i64) -> Result<Vec<ApiKeyRecord>, RepoError> {
        let rows = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, user_id, exchange, api_key, encrypted_secret, passphrase,
                   is_active, created_at
            FROM api_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query api keys: {}", e);
            RepoError::from(e)
        })?;

        Ok(rows)
    }

    #[instrument(skip(self), fields(user_id, exchange))]
    async fn find_by_exchange(
        &self,
        user_id: i64,
        exchange: &str,
    ) -> Result<Vec<ApiKeyRecord>, RepoError> {
        let rows = sqlx::query_as::<_, ApiKeyRecord>(
            r#"
            SELECT id, user_id, exchange, api_key, encrypted_secret, passphrase,
                   is_active, created_at
            FROM api_keys
            WHERE user_id = $1 AND exchange = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .bind(exchange)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to query api keys by exchange: {}", e);
            RepoError::from(e)
        })?;

        Ok(rows)
    }

    #[instrument(skip(self), fields(id, active))]
    async fn set_active(&self, id: i64, active: bool) -> Result<bool, RepoError> {
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET is_active = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(active)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update api key {}: {}", id, e);
            RepoError::from(e)
        })?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_api_key_roundtrip() {
        let row = NewApiKey {
            user_id: 1,
            exchange: "BINANCE".to_string(),
            api_key: "ak".to_string(),
            encrypted_secret: "cipher".to_string(),
            passphrase: Some("pp".to_string()),
        };
        assert_eq!(row.exchange, "BINANCE");
        assert_eq!(row.api_key, "ak");
        assert!(row.passphrase.is_some());
    }
}
