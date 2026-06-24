# Service 层编码 Spec（Rust）

> 业务服务 trait 定义及实现规范。

## Trait 定义规范

```rust
// crates/services/src/price.rs

/// 价格服务 trait
#[async_trait::async_trait]
pub trait PriceService: Send + Sync + 'static {
    /// 根据商品 ID 获取价格
    ///
    /// # Arguments
    /// * `item_id` - 商品 ID
    ///
    /// # Returns
    /// * `Ok(Some(Money))` - 查询到的价格
    /// * `Ok(None)` - 商品不存在
    /// * `Err(ServiceError)` - 服务异常
    async fn get_price(&self, item_id: ItemId) -> Result<Option<Money>, ServiceError>;

    /// 批量查询价格
    ///
    /// # Arguments
    /// * `item_ids` - 商品 ID 列表，最大 100
    ///
    /// # Returns
    /// * `Ok(HashMap<ItemId, Money>)` - item_id → price 映射
    async fn batch_get_price(
        &self,
        item_ids: &[ItemId],
    ) -> Result<HashMap<ItemId, Money>, ServiceError>;

    /// 创建价格规则
    async fn create_price_rule(
        &self,
        input: CreatePriceRuleInput,
    ) -> Result<PriceRule, ServiceError>;
}
```

## 实现规范

```rust
// crates/services/src/price.rs（继续）

/// 价格服务实现
#[derive(Clone)]
pub struct DefaultPriceService {
    repo: Arc<dyn PriceRuleRepository>,
    cache: Arc<dyn PriceCache>,
}

impl DefaultPriceService {
    pub fn new(repo: Arc<dyn PriceRuleRepository>, cache: Arc<dyn PriceCache>) -> Self {
        Self { repo, cache }
    }
}

#[async_trait::async_trait]
impl PriceService for DefaultPriceService {
    #[tracing::instrument(skip(self), fields(item_id = %item_id.0))]
    async fn get_price(&self, item_id: ItemId) -> Result<Option<Money>, ServiceError> {
        // 1. 查缓存
        if let Some(price) = self.cache.get_price(item_id).await? {
            tracing::debug!(price = price.as_cents(), "cache hit");
            return Ok(Some(price));
        }

        // 2. 查数据库
        let rows = self
            .repo
            .find_active_by_item_id(item_id, chrono::Utc::now().date_naive())
            .await
            .context("Failed to query price rules")?;

        // 3. 转换为领域对象
        let rules: Vec<PriceRule> = rows
            .into_iter()
            .map(PriceRule::try_from)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to convert price rules")?;

        // 4. 计算最优价格
        let price = rules
            .iter()
            .filter(|r| r.is_active())
            .min_by_key(|r| r.price)
            .map(|r| r.price);

        // 5. 写缓存
        if let Some(p) = price {
            self.cache.set_price(item_id, p).await?;
        }

        Ok(price)
    }
}
```

## 规范

### 1. Trait 设计

- 每个 Service 对应一个 trait + 至少一个实现
- trait 方法必须标注 `#[async_trait::async_trait]`
- trait 方法必须有文档注释（`///`），说明参数、返回值、错误
- trait 方法签名明确 `Option<T>` 表示"可能不存在"
- trait 方法签名中禁止裸 `bool` 返回值 — 使用语义化 enum
- trait 约束 `Send + Sync + 'static`

### 2. 实现规范

- 实现类命名 `DefaultXxxService`（可替换实现）
- 依赖通过构造函数注入（`Arc<dyn Trait>`），不使用全局变量
- 实现 `Clone`（因为 axum `State` 要求 `Clone`）
- 使用 `#[tracing::instrument]` 标注方法，记录业务上下文
- 多步骤业务操作使用事务（通过 Repository 的 transaction 方法）

### 3. 业务逻辑封装

- 核心业务逻辑封装在 Domain 层，Service 层做编排
- Service 不做纯数据转换工作（委托给 Domain 或 `From` 实现）
- 复杂多步骤流程使用状态机模式或 pipeline

### 4. 错误规范

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("repository error: {0}")]
    Repository(#[from] RepoError),

    #[error("cache error: {0}")]
    Cache(#[from] CacheError),

    #[error("business rule violation: {message}")]
    BusinessRule { message: String, context: String },

    #[error("not found: {entity}")]
    NotFound { entity: &'static str },

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}
```

- Service 层错误类型包含业务上下文
- 使用 `From` trait 实现下层错误的自动转换
- 业务异常消息包含可追踪的业务上下文 ID
- 禁止直接返回 `anyhow::Error`（使用 `ServiceError::Internal` 包装）

### 5. 性能约束

- 批量接口必须有大小限制（默认上限 100）
- 避免在循环中调用 Repository / Client
- 使用 `try_join!` / `join!` 并发执行独立操作
- 热点数据使用缓存 + Cache-Aside 模式

```rust
// 并发查询示例
use tokio::try_join;

let (price, inventory, promotion) = try_join!(
    self.price_service.get_price(item_id),
    self.inventory_service.get_stock(item_id),
    self.promotion_service.get_active_promotions(item_id),
)
.context("Failed to fetch product details")?;
```

### 6. 事务管理

- Service 层负责事务边界定义
- Repository 提供 `begin_transaction()` / `commit()` / `rollback()` 方法
- 或使用 closure-based 事务 API：

```rust
pub async fn execute_in_transaction<F, T>(&self, f: F) -> Result<T, ServiceError>
where
    F: FnOnce(&mut Transaction<'_, Postgres>) -> Future<Output = Result<T, RepoError>>,
{
    // 事务管理逻辑
}
```

### 7. 依赖注入

- Service 实现的所有依赖通过 trait object 注入
- 使用 `Arc<dyn Trait>` 实现共享所有权
- 在 AppState 中组装：

```rust
let repo = Arc::new(PgPriceRuleRepository::new(pool.clone()));
let cache = Arc::new(RedisPriceCache::new(redis.clone()));
let price_service = Arc::new(DefaultPriceService::new(repo, cache));

let state = AppState {
    price_service,
    // ...
};
```
