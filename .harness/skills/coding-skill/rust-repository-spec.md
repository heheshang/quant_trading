# Repository 层编码 Spec（Rust）

> 数据访问层规范：数据库表设计、sqlx 查询、Repository trait。

## 表结构设计规范

```sql
-- migrations/20260622000001_create_price_rules.sql

CREATE TABLE IF NOT EXISTS price_rule (
    id               BIGINT       NOT NULL GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    item_id          BIGINT       NOT NULL,
    price            BIGINT       NOT NULL,                -- 价格（单位：分）
    effective_start  DATE         NOT NULL,                -- 生效开始日期
    effective_end    DATE         NOT NULL,                -- 生效结束日期
    status           SMALLINT     NOT NULL DEFAULT 1,      -- 0-禁用 1-启用
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),  -- 创建时间
    updated_at       TIMESTAMPTZ  NOT NULL DEFAULT now(),  -- 修改时间
    version          INT          NOT NULL DEFAULT 1       -- 乐观锁版本号
);

-- 索引
CREATE INDEX idx_price_rule_item_id ON price_rule (item_id);
CREATE INDEX idx_price_rule_effective ON price_rule (effective_start, effective_end);
CREATE UNIQUE INDEX uk_price_rule_item_effective
    ON price_rule (item_id, effective_start)
    WHERE status = 1;

-- 触发器：自动更新 updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_price_rule_updated_at
    BEFORE UPDATE ON price_rule
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

## 数据库行类型

```rust
// crates/repository/src/models.rs

/// 数据库行类型 — 与表结构 1:1 映射
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PriceRuleRow {
    pub id: i64,
    pub item_id: i64,
    pub price: i64,
    pub effective_start: chrono::NaiveDate,
    pub effective_end: chrono::NaiveDate,
    pub status: i16,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub version: i32,
}

// 插入参数类型 — 只包含写入字段
#[derive(Debug)]
pub struct CreatePriceRuleParams {
    pub item_id: i64,
    pub price: i64,
    pub effective_start: chrono::NaiveDate,
    pub effective_end: chrono::NaiveDate,
    pub status: i16,
}
```

## Repository trait 定义

```rust
// crates/repository/src/price_repo.rs

/// 价格规则仓储 trait
///
/// trait 定义在 repository crate，可以被 service 层依赖。
/// 实现也在此 crate（Postgres 实现）或单独 crate。
#[async_trait::async_trait]
pub trait PriceRuleRepository: Send + Sync + 'static {
    /// 根据 ID 查询价格规则
    async fn find_by_id(
        &self,
        id: PriceRuleId,
    ) -> Result<Option<PriceRuleRow>, RepoError>;

    /// 根据商品 ID 查询有效规则
    async fn find_active_by_item_id(
        &self,
        item_id: ItemId,
        date: chrono::NaiveDate,
    ) -> Result<Vec<PriceRuleRow>, RepoError>;

    /// 批量创建价格规则
    async fn insert_batch(
        &self,
        params: &[CreatePriceRuleParams],
    ) -> Result<Vec<PriceRuleRow>, RepoError>;

    /// 更新价格规则（乐观锁）
    async fn update_with_version(
        &self,
        row: &PriceRuleRow,
    ) -> Result<PriceRuleRow, RepoError>;
}
```

## sqlx 实现

```rust
// crates/repository/src/price_repo.rs（继续）

pub struct PgPriceRuleRepository {
    pool: sqlx::PgPool,
}

impl PgPriceRuleRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl PriceRuleRepository for PgPriceRuleRepository {
    async fn find_by_id(
        &self,
        id: PriceRuleId,
    ) -> Result<Option<PriceRuleRow>, RepoError> {
        sqlx::query_as::<_, PriceRuleRow>(
            "SELECT id, item_id, price, effective_start, effective_end,
                    status, created_at, updated_at, version
             FROM price_rule
             WHERE id = $1"
        )
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(RepoError::from)
    }

    async fn find_active_by_item_id(
        &self,
        item_id: ItemId,
        date: chrono::NaiveDate,
    ) -> Result<Vec<PriceRuleRow>, RepoError> {
        sqlx::query_as::<_, PriceRuleRow>(
            "SELECT id, item_id, price, effective_start, effective_end,
                    status, created_at, updated_at, version
             FROM price_rule
             WHERE item_id = $1
               AND effective_start <= $2
               AND effective_end >= $2
               AND status = 1
             ORDER BY effective_start DESC"
        )
        .bind(item_id.0)
        .bind(date)
        .fetch_all(&self.pool)
        .await
        .map_err(RepoError::from)
    }
}
```

## 规范

### 1. 建表规范

- 表名小写蛇形，带业务前缀（`price_rule`）
- 必须包含 `created_at`、`updated_at` 字段（`TIMESTAMPTZ`）
- 建议包含 `version` 字段用于乐观锁
- 金额字段使用 `BIGINT`（单位：分），禁止 `DECIMAL` / `REAL` / `DOUBLE PRECISION`
- 状态字段使用 `SMALLINT`，对应 domain 层 enum
- 索引命名：`idx_{表名}_{字段名}`，唯一索引 `uk_{表名}_{字段名}`
- 字符串列避免 `TEXT` 无长度限制，使用 `VARCHAR(n)`

### 2. Row 类型规范

- Row 类型使用 `#[derive(sqlx::FromRow)]`
- Row 字段名与数据库列名一致（蛇形）
- Row 类型不包含业务逻辑（纯数据载体）
- Row → Domain 转换在 repository crate 中实现 `From<PriceRuleRow> for PriceRule`
- Domain → Row/Params 转换也在 repository crate 中

### 3. Repository trait 规范

- trait 方法使用 `#[async_trait::async_trait]`
- 方法命名遵循：`find_by_*`（查询）、`insert_*`（创建）、`update_*`（更新）、`delete_*`（删除）
- 返回类型使用 `Result<_, RepoError>`
- 查询不存在时返回 `Option<T>`，不返回错误
- 所有 trait 方法必须是 async

### 4. 查询规范

- 禁止 `SELECT *` — 显式列出所有列
- 使用 `$1, $2` 参数化查询，禁止字符串拼接 SQL
- 批量操作使用 `UNNEST` 或批量绑定（sqlx 支持）
- 分页查询必须指定 `LIMIT` 和 `OFFSET`
- 复杂查询使用 CTE（`WITH` 子句）
- 执行 `cargo sqlx prepare` 并将 `.sqlx/` 提交到 VCS

### 5. 写入规范

- 插入使用 `RETURNING *` 返回完整行
- 更新使用乐观锁（`WHERE version = $n`）
- 禁止物理删除 — 使用 `is_deleted` 或状态字段标记
- 批量操作使用事务包裹

```rust
// 批量插入示例
pub async fn insert_batch(
    &self,
    params: &[CreatePriceRuleParams],
) -> Result<Vec<PriceRuleRow>, RepoError> {
    let mut tx = self.pool.begin().await?;
    let mut rows = Vec::with_capacity(params.len());

    for param in params {
        let row = sqlx::query_as::<_, PriceRuleRow>(
            "INSERT INTO price_rule (item_id, price, effective_start, effective_end, status)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, item_id, price, effective_start, effective_end,
                       status, created_at, updated_at, version"
        )
        .bind(param.item_id)
        .bind(param.price)
        .bind(param.effective_start)
        .bind(param.effective_end)
        .bind(param.status)
        .fetch_one(&mut *tx)
        .await?;
        rows.push(row);
    }

    tx.commit().await?;
    Ok(rows)
}
```

### 6. 错误类型

```rust
// crates/repository/src/errors.rs

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("record not found: {entity} id={id}")]
    NotFound { entity: &'static str, id: String },

    #[error("optimistic lock conflict: {entity} id={id}, version={version}")]
    VersionConflict {
        entity: &'static str,
        id: String,
        version: i32,
    },
}
```

### 7. 禁止的操作

- 禁止在 handler / service 中直接使用 `sqlx::query*` — 必须通过 Repository trait
- 禁止在 Mapper 逻辑中使用 `SELECT *`
- 禁止在循环中逐条执行 SQL（使用批量操作）
- 禁止跨数据库 JOIN（应用层组装）
- 禁止使用同步数据库驱动（如 `rusqlite`）在 async 上下文中
