use quant_common::config::DatabaseConfig;
use quant_common::error::Error;
use quant_common::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument};

/// PostgreSQL 数据库客户端
///
/// Connection pool wrapper used by per-entity repositories. Migration management
/// is owned by `data_layer::PostgresClient`; this pool only serves queries.
pub struct PostgresClient {
    pool: Arc<PgPool>,
}

impl PostgresClient {
    /// Create a new PostgreSQL connection pool from config.
    #[instrument(skip(config), fields(database = %config.database, host = %config.host))]
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let connection_string = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.username, config.password, config.host, config.port, config.database
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .connect(&connection_string)
            .await
            .map_err(|e| {
                error!("Database connection failed: {}", e);
                Error::Database(e.to_string())
            })?;

        info!("Database connected successfully");
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Wrap an existing PostgreSQL pool.
    ///
    /// Used by the application entrypoint to avoid opening a second connection
    /// pool when `data_layer::PostgresClient` already owns one.
    pub fn from_pool(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// Health check — verifies database connectivity.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<bool> {
        sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await
            .map(|_| true)
            .map_err(|e| {
                error!("Health check failed: {}", e);
                Error::Database(e.to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a running PostgreSQL instance"]
    async fn test_connection() {
        // Config is loaded from env via the centralized dotenv-based loader.
        let config = quant_common::config::AppConfig::from_env().database;
        let client = PostgresClient::new(&config).await;
        // This test requires a running PostgreSQL instance
        assert!(client.is_ok());
    }
}
