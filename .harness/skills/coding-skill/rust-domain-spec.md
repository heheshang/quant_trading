# Domain 层编码 Spec（Rust）

> 领域模型与业务逻辑封装规范。

## 领域模型设计

```rust
// crates/domain/src/price.rs

/// 价格规则
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRule {
    /// 规则 ID
    pub id: PriceRuleId,
    /// 商品 ID
    pub item_id: ItemId,
    /// 价格（单位：分）
    pub price: Money,
    /// 生效时间范围
    pub effective_period: DateTimeRange,
    /// 状态
    pub status: PriceRuleStatus,
}

/// 金额（单位：分，i64 保证精确）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(i64);

impl Money {
    /// 从分创建
    pub const fn from_cents(cents: i64) -> Self {
        Self(cents)
    }

    /// 以分形式获取金额
    pub const fn as_cents(&self) -> i64 {
        self.0
    }

    /// 是否为非负数
    pub fn is_non_negative(&self) -> bool {
        self.0 >= 0
    }
}

/// 价格规则状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceRuleStatus {
    /// 禁用
    Disabled,
    /// 启用
    Enabled,
}

impl PriceRule {
    /// 判断规则在指定日期是否生效
    pub fn is_effective_at(&self, date: chrono::NaiveDate) -> bool {
        self.effective_period.contains(date)
    }

    /// 判断规则当前是否激活
    pub fn is_active(&self) -> bool {
        self.status == PriceRuleStatus::Enabled
    }
}
```

## 规范

### 1. 领域对象定位

- Domain 对象是"富模型"：数据 + 行为
- 与数据库行类型（`PriceRuleRow`）严格区分：Row 是数据载体，Domain 是业务实体
- Domain 对象不包含序列化/反序列化逻辑（那是 Row/DTO 的职责）
- 使用 Newtype 模式包装原始类型（`Money(i64)`, `ItemId(i64)`）

### 2. 封装原则

- 字段使用 `pub(crate)` 或私有 + getter
- 业务方法对外 `pub`，内部状态不对外泄露
- 构造器确保创建即合法（非法状态不可构造）
- 禁止 `pub` 暴露内部可变引用

### 3. Newtype 模式（必用）

每个业务 ID 和值对象必须使用 Newtype 包装：

```rust
/// 商品 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId(pub i64);

/// 价格规则 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PriceRuleId(pub i64);

/// 用户 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub uuid::Uuid);
```

### 4. 业务规则编码方法

- 业务规则编码为领域方法（如 `is_effective_at()`, `can_apply()`）
- 方法命名清晰表达业务含义
- 复杂规则拆分为多个小方法
- 使用 `#[must_use]` 标注纯查询方法
- 布尔返回值禁止裸 `bool`，考虑使用语义化 enum（`Decision::Allow` / `Decision::Deny`）

### 5. 值对象

- 值对象实现 `Copy`（如果语义上是值语义）
- 值对象实现 `PartialEq + Eq`（值比较）
- 值对象实现 `Debug`（不含敏感信息）
- 值对象构造器验证合法性（`new()` 返回 `Result` 或使用 `TryFrom`）

```rust
/// 折扣率（0-10000 表示 0%-100%，basis points）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiscountRate(u32);

impl DiscountRate {
    pub fn new(basis_points: u32) -> Result<Self, DomainError> {
        if basis_points > 10000 {
            return Err(DomainError::InvalidDiscountRate(basis_points));
        }
        Ok(Self(basis_points))
    }
}
```

### 6. 禁止的行为

- Domain 不依赖任何外部 crate（sqlx / reqwest / redis 等）
- Domain 不依赖 `AppState` 或任何 DI 容器
- Domain 不做 IO 操作（网络 / 文件 / 数据库）
- Domain 不包含异步函数（保持纯同步，纯计算）
- Domain 不 panic — 所有失败通过 `Result` 返回
- Domain 不使用 `Serialize` / `Deserialize` derive（数据转换在 Row/DTO 层）
