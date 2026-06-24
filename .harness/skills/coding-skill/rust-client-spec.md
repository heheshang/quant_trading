# Client 层编码 Spec（Rust）

> 外部服务调用规范：HTTP 调用、gRPC 调用、缓存操作、消息发送。

## HTTP Client 调用规范

```rust
// crates/clients/src/external_price.rs

/// 外部价格服务客户端 trait
#[async_trait::async_trait]
pub trait ExternalPriceClient: Send + Sync + 'static {
    /// 查询外部价格
    async fn query_price(&self, item_id: ItemId) -> Result<Option<ExternalPrice>, ClientError>;
}

/// 基于 reqwest 的实现
#[derive(Clone)]
pub struct ReqwestExternalPriceClient {
    client: reqwest::Client,
    base_url: String,
}

impl ReqwestExternalPriceClient {
    pub fn new(base_url: String, timeout: std::time::Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build reqwest client");

        Self { client, base_url }
    }
}

#[async_trait::async_trait]
impl ExternalPriceClient for ReqwestExternalPriceClient {
    #[tracing::instrument(skip(self), fields(item_id = %item_id.0))]
    async fn query_price(&self, item_id: ItemId) -> Result<Option<ExternalPrice>, ClientError> {
        let url = format!("{}/api/v1/prices/{}", self.base_url, item_id.0);

        let response = self
            .client
            .get(&url)
            .header("X-Request-Id", uuid::Uuid::new_v4().to_string())
            .send()
            .await
            .context("HTTP request failed")?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let body: ExternalPriceResponse = response
                    .json()
                    .await
                    .context("Failed to parse response")?;
                Ok(Some(ExternalPrice::from(body)))
            }
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            status => {
                tracing::warn!(
                    status = %status,
                    item_id = %item_id.0,
                    "External price service returned unexpected status"
                );
                Ok(None) // 降级返回空
            }
        }
    }
}
```

## gRPC Client 调用规范

```rust
// crates/clients/src/price_grpc.rs

use tonic::transport::Channel;

/// gRPC 价格服务客户端
#[derive(Clone)]
pub struct GrpcPriceClient {
    client: price_service_client::PriceServiceClient<Channel>,
}

impl GrpcPriceClient {
    pub async fn connect(addr: &str) -> Result<Self, ClientError> {
        let channel = Channel::from_shared(addr.to_string())
            .context("Invalid gRPC address")?
            .connect_timeout(std::time::Duration::from_secs(5))
            .connect()
            .await
            .context("Failed to connect to gRPC server")?;

        Ok(Self {
            client: price_service_client::PriceServiceClient::new(channel),
        })
    }
}

#[async_trait::async_trait]
impl ExternalPriceClient for GrpcPriceClient {
    async fn query_price(&self, item_id: ItemId) -> Result<Option<ExternalPrice>, ClientError> {
        let request = tonic::Request::new(QueryPriceRequest {
            item_id: item_id.0,
        });

        let response = self
            .client
            .clone()
            .query_price(request)
            .await
            .context("gRPC call failed")?
            .into_inner();

        // ... convert proto to domain
        todo!()
    }
}
```

## 缓存操作规范

```rust
// crates/clients/src/cache.rs

/// 缓存操作 trait
#[async_trait::async_trait]
pub trait PriceCache: Send + Sync + 'static {
    /// 获取缓存价格
    async fn get_price(&self, item_id: ItemId) -> Result<Option<Money>, ClientError>;

    /// 设置缓存价格
    async fn set_price(&self, item_id: ItemId, price: Money) -> Result<(), ClientError>;

    /// 批量删除缓存
    async fn delete_prices(&self, item_ids: &[ItemId]) -> Result<(), ClientError>;
}

/// Redis 实现
#[derive(Clone)]
pub struct RedisPriceCache {
    redis: Arc<redis::aio::MultiplexedConnection>,
}

impl RedisPriceCache {
    const KEY_PREFIX: &'static str = "price";
    const DEFAULT_TTL: u64 = 3600; // 1 小时

    fn cache_key(item_id: ItemId) -> String {
        format!("{}:{}:{}", Self::KEY_PREFIX, "item", item_id.0)
    }
}

#[async_trait::async_trait]
impl PriceCache for RedisPriceCache {
    async fn get_price(&self, item_id: ItemId) -> Result<Option<Money>, ClientError> {
        let key = Self::cache_key(item_id);
        let value: Option<i64> = self
            .redis
            .clone()
            .get(&key)
            .await
            .context("Redis GET failed")?;

        Ok(value.map(Money::from_cents))
    }

    async fn set_price(&self, item_id: ItemId, price: Money) -> Result<(), ClientError> {
        let key = Self::cache_key(item_id);
        redis::pipe()
            .set(&key, price.as_cents())
            .expire(&key, Self::DEFAULT_TTL as i64)
            .query_async(&mut *self.redis.clone())
            .await
            .context("Redis SET failed")?;

        Ok(())
    }

    async fn delete_prices(&self, item_ids: &[ItemId]) -> Result<(), ClientError> {
        if item_ids.is_empty() {
            return Ok(());
        }

        let keys: Vec<String> = item_ids.iter().map(|id| Self::cache_key(*id)).collect();

        self.redis
            .clone()
            .del(keys)
            .await
            .context("Redis DEL failed")?;

        Ok(())
    }
}
```

## 消息发送规范

```rust
// crates/clients/src/message_queue.rs

/// 消息生产者 trait
#[async_trait::async_trait]
pub trait MessageProducer: Send + Sync + 'static {
    /// 发送价格变更事件
    async fn publish_price_changed(
        &self,
        event: PriceChangedEvent,
    ) -> Result<(), ClientError>;
}

/// 价格变更事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChangedEvent {
    /// 事件 ID（幂等去重）
    pub event_id: uuid::Uuid,
    /// 商品 ID
    pub item_id: i64,
    /// 旧价格（分）
    pub old_price: Option<i64>,
    /// 新价格（分）
    pub new_price: i64,
    /// 事件时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

## 规范

### 1. 统一封装

- 所有外部调用封装在 `clients` crate 中
- 每个外部服务一个 module
- 必须定义 trait 以便 Mock 测试
- Service 层不直接调用 `reqwest` / `redis` / `tonic`，必须通过 Client trait

### 2. 超时设置

- HTTP 调用：必须设置 `connect_timeout` + `timeout`
- gRPC 调用：设置 `connect_timeout` + 请求级 deadline
- 超时时间根据接口 P99 响应时间设定（1.5× ~ 2× P99）
- 长耗时操作考虑异步 + 回调模式

### 3. 降级策略

- 所有外部调用必须有降级逻辑
- 降级返回：默认值 / 缓存值 / `None` / 空列表
- 降级时打印 WARN 级别日志（含降级原因）
- 批量调用中单条失败不影响其他结果
- 降级行为可配置（熔断阈值、降级策略）

### 4. 重试策略

- 幂等 GET 请求可重试（最多 3 次，指数退避）
- 非幂等 POST/PUT/DELETE 禁止重试
- 使用 `tower::retry::RetryLayer` 或手动实现
- 重试必须有上限和退避策略
- 重试失败计入熔断器

### 5. 熔断器

- 使用 `tower` 或 `circuit-breaker` crate
- 熔断条件：连续失败 N 次或错误率超过阈值
- 半开状态：定期探测服务恢复
- 熔断时所有请求直接走降级

### 6. 缓存操作规范

- 缓存 Key 格式：`{业务前缀}:{实体}:{ID}`
- 先更新 DB，再删除缓存（Cache-Aside Pattern）
- 批量淘汰使用 pipeline / mget / mset
- 设置合理的 TTL（默认 1 小时，热点数据可延长）
- 缓存不可用时降级到直接查 DB

### 7. 消息发送规范

- 消息体必须包含唯一 `event_id`（用于消费者去重）
- 消息必须声明 TTL
- 关键业务事件使用事务消息或 Outbox 模式
- 消息序列化使用 JSON（`serde_json`）或 Protobuf（`prost`）

### 8. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("gRPC call failed: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("cache operation failed: {0}")]
    Cache(String),

    #[error("message publish failed: {0}")]
    MessageQueue(String),

    #[error("circuit breaker is open")]
    CircuitBreakerOpen,

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}
```

### 9. 禁止的操作

- 禁止在 handler / service 中直接使用 `reqwest::get()` — 必须通过 Client trait
- 禁止忽略外部调用的错误（至少打印 WARN 日志）
- 禁止无限重试（必须设置上限）
- 禁止在 async 中使用同步 HTTP 客户端
- 禁止硬编码外部服务地址（通过配置注入）
