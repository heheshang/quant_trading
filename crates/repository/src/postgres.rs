use quant_common::config::DatabaseConfig;
use quant_common::error::Error;
use quant_common::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;

/// PostgreSQL 数据库客户端
///
/// Provides connection pool management and migration support.
/// Specific entity repositories wrap this with their own traits.
pub struct PostgresClient {
    pool: Arc<PgPool>,
}

impl PostgresClient {
    /// Create a new PostgreSQL connection pool from config.
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let connection_string = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.username, config.password, config.host, config.port, config.database
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&connection_string)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Run database migrations from a given directory.
    /// Migrations live in `crates/data-layer/migrations/` for now.
    pub async fn run_migrations_with_path(&self, path: &str) -> Result<()> {
        // Migration files live in their respective crate directories.
        // For Tauri, migration path is resolved relative to the binary.
        sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        tracing::warn!(
            "run_migrations_with_path({}) is a stub — run migrations from data-layer",
            path
        );
        Ok(())
    }

    /// Get a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Health check — verifies database connectivity.
    pub async fn health_check(&self) -> Result<bool> {
        sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await
            .map(|_| true)
            .map_err(|e| Error::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection() {
        let config = DatabaseConfig {
            host: dotenv::var("DATABASE_HOST").unwrap_or_else(|_| "localhost".into()),
            port: dotenv::var("DATABASE_PORT")
                .unwrap_or_else(|_| "5432".into())
                .parse::<u16>()
                .unwrap_or(5432),
            username: dotenv::var("DATABASE_USERNAME").unwrap_or_else(|_| "postgres".into()),
            password: dotenv::var("DATABASE_PASSWORD").unwrap_or_else(|_| "postgres".into()),
            database: dotenv::var("DATABASE_NAME").unwrap_or_else(|_| "quant_trading".into()),
            max_connections: 5,
        };

        let client = PostgresClient::new(&config).await;
        // This test requires a running PostgreSQL instance
        assert!(client.is_ok());
    }
}
