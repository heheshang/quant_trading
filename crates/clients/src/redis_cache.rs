use deadpool_redis::{Config, Pool, Runtime};
use quant_common::config::RedisConfig;
use quant_common::{Error, Result};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Redis 缓存客户端
pub struct RedisCache {
    pool: Pool,
}

impl RedisCache {
    /// Create a new Redis cache client from config.
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
            .map_err(|e| {
                error!("Failed to create Redis pool: {}", e);
                Error::Redis(e.to_string())
            })?;

        info!("Redis cache connected to {}:{}/{}", config.host, config.port, config.db);
        Ok(Self { pool })
    }

    /// Set a key-value pair with optional TTL.
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        debug!("Cache set key={}", key);
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                error!("Cache set failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        let serialized =
            serde_json::to_string(value).map_err(|e| Error::Internal(e.to_string()))?;

        if let Some(ttl) = ttl {
            let _: () = conn
                .set_ex(key, serialized, ttl.as_secs())
                .await
                .map_err(|e| {
                    error!("Cache set_ex failed for key={}: {}", key, e);
                    Error::Redis(e.to_string())
                })?;
        } else {
            let _: () = conn
                .set(key, serialized)
                .await
                .map_err(|e| {
                    error!("Cache set failed for key={}: {}", key, e);
                    Error::Redis(e.to_string())
                })?;
        }

        Ok(())
    }

    /// Get a value by key.
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        debug!("Cache get key={}", key);
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                error!("Cache get failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        let value: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| {
                error!("Cache get failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        match value {
            Some(v) => {
                let deserialized =
                    serde_json::from_str(&v).map_err(|e| Error::Internal(e.to_string()))?;
                Ok(Some(deserialized))
            }
            None => {
                warn!("Cache miss for key={}", key);
                Ok(None)
            }
        }
    }

    /// Delete a key.
    pub async fn delete(&self, key: &str) -> Result<()> {
        debug!("Cache delete key={}", key);
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                error!("Cache delete failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        let _: () = conn
            .del(key)
            .await
            .map_err(|e| {
                error!("Cache delete failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        Ok(())
    }

    /// Check if a key exists.
    pub async fn exists(&self, key: &str) -> Result<bool> {
        debug!("Cache exists key={}", key);
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                error!("Cache exists failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| {
                error!("Cache exists failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        Ok(exists)
    }

    /// Set with explicit expiry in seconds.
    pub async fn set_with_expiry(&self, key: &str, value: &str, seconds: u64) -> Result<()> {
        debug!("Cache set_with_expiry key={}", key);
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                error!("Cache set_with_expiry failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        let _: () = conn
            .set_ex(key, value, seconds)
            .await
            .map_err(|e| {
                error!("Cache set_with_expiry failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        Ok(())
    }

    /// Increment a counter.
    pub async fn increment(&self, key: &str) -> Result<i64> {
        debug!("Cache increment key={}", key);
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                error!("Cache increment failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        let value: i64 = conn
            .incr(key, 1)
            .await
            .map_err(|e| {
                error!("Cache increment failed for key={}: {}", key, e);
                Error::Redis(e.to_string())
            })?;

        Ok(value)
    }

    /// Health check via PING.
    pub async fn health_check(&self) -> Result<bool> {
        debug!("Cache health_check");
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| {
                error!("Cache health_check failed: {}", e);
                Error::Redis(e.to_string())
            })?;

        let pong: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                error!("Cache health_check PING failed: {}", e);
                Error::Redis(e.to_string())
            })?;

        Ok(pong == "PONG")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_operations() {
        let _config = RedisConfig {
            host: "localhost".to_string(),
            port: 6379,
            password: None,
            db: 0,
            pool_size: 5,
        };

        // Requires a running Redis instance
        // let cache = RedisCache::new(&config).unwrap();
        // cache.set("test_key", &"test_value".to_string(), None).await.unwrap();
        // let value: Option<String> = cache.get("test_key").await.unwrap();
        // assert_eq!(value, Some("test_value".to_string()));
    }
}
