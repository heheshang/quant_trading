use quant_common::config::DatabaseConfig;
use quant_common::{Error, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, instrument};

/// PostgreSQL 数据库客户端
pub struct PostgresClient {
    pool: Arc<PgPool>,
}

impl PostgresClient {
    /// 创建新的数据库连接
    #[instrument]
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .connect(&config.connection_string())
            .await
            .map_err(|e| {
                error!("Failed to connect to database: {}", e);
                Error::Database(e.to_string())
            })?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// 创建连接池但不阻塞启动，连接会在首次使用或后台迁移时建立。
    #[instrument]
    pub fn new_lazy(config: &DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .connect_lazy(&config.connection_string())
            .map_err(|e| {
                error!("Failed to configure database connection pool: {}", e);
                Error::Database(e.to_string())
            })?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// 运行数据库迁移
    #[instrument(skip(self))]
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&*self.pool)
            .await
            .map_err(|e| Error::Database(format!("Migration failed: {}", e)))?;
        Ok(())
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 包装已有的连接池,避免为同一数据库打开第二套连接池。
    pub fn from_pool(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    /// 健康检查
    #[instrument(skip(self))]
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
    #[ignore = "requires a running PostgreSQL instance"]
    async fn test_connection() {
        // Config is loaded from env via the centralized dotenv-based loader.
        let config = quant_common::config::AppConfig::from_env().database;

        // This test requires a running PostgreSQL instance
        // Uncomment when database is available
        let client = PostgresClient::new(&config).await;
        assert!(client.is_ok());
    }
}
