use quant_common::config::DatabaseConfig;
use quant_common::{Error, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;

/// PostgreSQL 数据库客户端
pub struct PostgresClient {
    pool: Arc<PgPool>,
}

impl PostgresClient {
    /// 创建新的数据库连接
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

    /// 运行数据库迁移
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&*self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// 获取连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 健康检查
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
    use dotenv::dotenv;
    #[tokio::test]
    async fn test_connection() {
        let config = DatabaseConfig {
            host: dotenv::var("DATABASE_HOST").unwrap(),
            port: dotenv::var("DATABASE_PORT")
                .unwrap()
                .parse::<u16>()
                .unwrap(),
            username: dotenv::var("DATABASE_USERNAME").unwrap(),
            password: dotenv::var("DATABASE_PASSWORD").unwrap(),
            database: dotenv::var("DATABASE_NAME").unwrap(),
            max_connections: 5,
        };

        // This test requires a running PostgreSQL instance
        // Uncomment when database is available
        let client = PostgresClient::new(&config).await;
        assert!(client.is_ok());
    }
}
