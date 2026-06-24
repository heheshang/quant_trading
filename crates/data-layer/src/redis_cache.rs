use deadpool_redis::{Config, Pool, Runtime};
use quant_common::config::RedisConfig;
use quant_common::{Error, Result};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Redis 缓存客户端
pub struct RedisCache {
    pool: Pool,
}

impl RedisCache {
    /// 创建新的 Redis 缓存客户端
    pub fn new(config: &RedisConfig) -> Result<Self> {
        let redis_url = if let Some(password) = &config.password {
            format!(
                "redis://:{}@{}:{}/{}",
                password, config.host, config.port, config.db
            )
        } else {
            format!("redis://{}:{}/{}", config.host, config.port, config.db)
        };

        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(Self { pool })
    }

    /// 设置键值对
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let serialized =
            serde_json::to_string(value).map_err(|e| Error::Internal(e.to_string()))?;

        if let Some(ttl) = ttl {
            let _: () = conn
                .set_ex(key, serialized, ttl.as_secs())
                .await
                .map_err(|e| Error::Redis(e.to_string()))?;
        } else {
            let _: () = conn
                .set(key, serialized)
                .await
                .map_err(|e| Error::Redis(e.to_string()))?;
        }

        Ok(())
    }

    /// 获取键值
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let value: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        match value {
            Some(v) => {
                let deserialized =
                    serde_json::from_str(&v).map_err(|e| Error::Internal(e.to_string()))?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    /// 删除键
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let _: () = conn
            .del(key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(())
    }

    /// 检查键是否存在
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(exists)
    }

    /// 设置带过期时间的键值对（秒）
    pub async fn set_with_expiry(&self, key: &str, value: &str, seconds: u64) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let _: () = conn
            .set_ex(key, value, seconds)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(())
    }

    /// 增加计数器
    pub async fn increment(&self, key: &str) -> Result<i64> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let value: i64 = conn
            .incr(key, 1)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(value)
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        let pong: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| Error::Redis(e.to_string()))?;

        Ok(pong == "PONG")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_operations() {
        let _config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            db: 0,
            pool_size: 5,
        };

        // This test requires a running Redis instance
        // Uncomment when Redis is available
        // let cache = RedisCache::new(&config).unwrap();
        // cache.set("test_key", &"test_value".to_string(), None).await.unwrap();
        // let value: Option<String> = cache.get("test_key").await.unwrap();
        // assert_eq!(value, Some("test_value".to_string()));
    }
}
