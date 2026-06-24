# Custom Linter Rules — Rust（自定义 Lint 规则）

> 参考：[OpenAI: Harness engineering - leveraging Codex in an agent-first world]
>
> "We enforce these rules with custom linters and structural tests,
> plus a small set of 'taste invariants.'"
>
> 可机械执行的架构约束 → 比写在文档里的规则更有效。
> Agent 可能忽略 README，但不能忽略编译失败。

## 规则类型

### 类型 A：编译期检查（最可靠）

在 `Cargo.toml`（`[workspace.lints.clippy]`）或源码属性（`#![forbid(...)]`）中直接嵌入检查，违反即编译失败。

### 类型 B：测试期检查

在 CI 中和单元测试一起运行，违反即测试失败。使用 `#[cfg(test)]` 模块中的断言或 `trybuild` crate 验证编译期失败。

### 类型 C：CI 门禁检查

在 CI pipeline 中独立步骤运行（`cargo clippy`、`cargo fmt --check`、`cargo deny`、`cargo audit`）。

### 类型 D：定期扫描检查

由 Entropy GC 定期扫描，不阻塞 CI 但生成 PR。

## 示例规则

### LINT-001：禁止使用 f64/f32 表示金额

```rust
// ❌ 错误
let price: f64 = 99.99;
let total = price * quantity as f64;

// ✅ 正确
let price = Money::from_cents(9999);  // i64 内部表示
let total = price * quantity;          // Money(i64) * i64 -> Money(i64)
```

**检查方式：**
```toml
# Cargo.toml — workspace 级别
[workspace.lints.clippy]
# 无直接 clippy 规则，但可通过 disallowed-types 强制
```

```rust
// crates/domain/src/lib.rs
#![deny(clippy::disallowed_types)]

// clippy.toml
disallowed-types = [
    { path = "f64", reason = "use Money(i64) for monetary values" },
    { path = "f32", reason = "use Money(i64) for monetary values" },
]
```

**执行：** `cargo clippy -- -D warnings`

### LINT-002：所有外部调用必须设置超时

```rust
// ❌ 错误 — 无超时，可能永久阻塞
async fn fetch_price(client: &reqwest::Client, url: &str) -> Result<Price> {
    let resp = client.get(url).send().await?.json().await?;
    Ok(resp)
}

// ✅ 正确 — 显式超时 + 降级
async fn fetch_price(client: &reqwest::Client, url: &str) -> Result<Price> {
    let resp = client
        .get(url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .context("price fetch timeout")?
        .json()
        .await
        .context("price decode failed")?;
    Ok(resp)
}
```

**检查方式：**
```rust
// tests/lint_test.rs — 测试期检查
#[test]
fn test_all_http_calls_have_timeout() {
    // 使用 syn 解析源码，检查所有 .send().await 调用前是否有 .timeout()
    // 或使用 grep-based 扫描
}
```

**执行：** `cargo test --test lint_test`

### LINT-003：禁止循环依赖（crate 之间）

```rust
// ❌ 错误 — services 依赖 repository，repository 又依赖 services
// crates/services/Cargo.toml
//   my-service-repository = { path = "../repository" }
// crates/repository/Cargo.toml
//   my-service-services = { path = "../services" }  // 循环！

// ✅ 正确 — repository 依赖 domain，services 依赖 repository + domain
// crates/repository/Cargo.toml
//   my-service-domain = { path = "../domain" }
// crates/services/Cargo.toml
//   my-service-domain = { path = "../domain" }
//   my-service-repository = { path = "../repository" }
```

**检查方式：**
```bash
# CI 门禁 — cargo-deny 检查 bans 段
cargo deny check bans

# 或自定义脚本解析 Cargo.toml 依赖图
# 检查规则：app → handlers → services → domain（单向）
```

**执行：** `cargo deny check`

### LINT-004：Handler 层不能直接依赖 Repository

```rust
// ❌ 错误 — 跨层调用
// crates/handlers/Cargo.toml
//   my-service-repository = { path = "../repository" }  // 跨层！

// crates/handlers/src/price.rs
use my_service_repository::PriceRepository;  // handler 直接访问 Repository

async fn get_price(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Price>> {
    let price = state.price_repo.find_by_id(id).await?;  // 跨层！
    Ok(Json(price))
}

// ✅ 正确 — 通过 service 层
// crates/handlers/Cargo.toml
//   my-service-services = { path = "../services" }  // 只依赖 service 层

async fn get_price(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Price>> {
    let price = state.price_service.get_price(id).await?;  // 通过 service
    Ok(Json(price))
}
```

**检查方式：**
```bash
# 脚本检查 — 解析 crates/handlers/Cargo.toml，禁止出现 repository 依赖
grep -q "my-service-repository" crates/handlers/Cargo.toml && exit 1
```

**执行：** CI lint 步骤

### LINT-005：禁止 unwrap() / expect() 在生产代码中

```rust
// ❌ 错误 — 运行时 panic 风险
fn parse_price(s: &str) -> Money {
    let cents: i64 = s.parse().unwrap();  // panic on invalid input!
    Money::from_cents(cents)
}

let price = prices.get(0).unwrap();  // panic on empty slice!

// ✅ 正确 — 错误传播
fn parse_price(s: &str) -> Result<Money, ParseError> {
    let cents: i64 = s.parse().context("invalid price format")?;
    Ok(Money::from_cents(cents))
}

let price = prices.first().context("price list is empty")?;
```

**检查方式：**
```toml
# Cargo.toml — workspace 级别 clippy
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic_in_result_fn = "deny"
```

**执行：** `cargo clippy --all-targets -- -D warnings`

### LINT-006：domain 层零外部依赖

```rust
// ❌ 错误 — domain crate 引入 sqlx
// crates/domain/Cargo.toml
//   sqlx = { workspace = true }  // domain 不应依赖基础设施！

// crates/domain/src/price.rs
use sqlx::FromRow;  // 基础设施泄漏到领域层！

#[derive(FromRow)]
pub struct Price {
    pub id: i64,
    pub amount: i64,
}

// ✅ 正确 — domain 仅定义 trait，实现在 repository 层
// crates/domain/Cargo.toml
//   [dependencies]
//   # 无 sqlx、reqwest 等基础设施依赖
//   serde = { workspace = true }  # 仅允许 serde 等通用库

// crates/domain/src/price.rs
pub struct Price {
    pub id: PriceId,
    pub amount: Money,
}

pub trait PriceRepository: Send + Sync {
    async fn find_by_id(&self, id: &PriceId) -> Result<Option<Price>, RepoError>;
}
```

**检查方式：**
```bash
# 脚本检查 — crates/domain/Cargo.toml 不含 sqlx/reqwest/redis 等基础设施依赖
forbidden_deps=("sqlx" "reqwest" "redis" "tokio")
for dep in "${forbidden_deps[@]}"; do
    grep -q "^$dep" crates/domain/Cargo.toml && exit 1
done
```

**执行：** CI lint 步骤

### LINT-007：所有 async fn 必须有 tracing instrument

```rust
// ❌ 错误 — 无 span，生产环境无法追踪
async fn calculate_price(item_id: ItemId, qty: u32) -> Result<Money> {
    // ... 业务逻辑
}

// ✅ 正确 — 自动创建 span
#[tracing::instrument(skip(self), fields(item_id = %item_id))]
async fn calculate_price(&self, item_id: ItemId, qty: u32) -> Result<Money> {
    // ... 业务逻辑
}
```

**检查方式：**
```bash
# 脚本检查 — 所有 async fn 必须上方有 #[tracing::instrument]
# 使用 ripgrep + AST 解析
rg "async fn" --type rust -l | while read f; do
    # 检查每个 async fn 上方是否有 #[tracing::instrument]
done
```

**执行：** CI lint 步骤（Entropy GC 定期扫描）

### LINT-008：禁止 unsafe 代码（非 FFI crate）

```rust
// ❌ 错误 — 无安全说明的 unsafe
unsafe {
    *ptr = 42;  // 无 SAFETY 注释
}

// ✅ 正确 A — 完全禁止
// crates/domain/src/lib.rs
#![forbid(unsafe_code)]

// ✅ 正确 B — 必须有 SAFETY 注释（仅限 FFI crate）
// SAFETY: `ptr` is valid because it was obtained from a Box and not freed.
unsafe {
    *ptr = 42;
}
```

**检查方式：**
```rust
// 非 FFI crate 的 lib.rs 中添加
#![forbid(unsafe_code)]
```

**执行：** `cargo clippy -- -D warnings`（编译期强制）

### LINT-009：禁止 `#[allow(...)]` 全局压制

```rust
// ❌ 错误 — 全局压制，隐藏所有警告
// crates/services/src/lib.rs
#![allow(clippy::all)]

// ✅ 正确 A — 行级 allow，明确范围
#[allow(clippy::too_many_arguments)]
fn complex_handler(
    state: State,
    id: i64,
    name: String,
    // ...
) -> Result<()> { }

// ✅ 正确 B — 在 Cargo.toml 中精确配置
[workspace.lints.clippy]
module_name_repetitions = "allow"  # 团队评审后的明确例外
```

**检查方式：**
```bash
# 检查 lib.rs / main.rs 中无 #![allow(...)]
rg "^#!\[allow" --type rust && exit 1
```

**执行：** CI lint 步骤

### LINT-010：sqlx 查询必须使用 prepare 缓存

```rust
// ❌ 错误 — 运行时编译 SQL，CI 无法离线检查
let price = sqlx::query_as::<_, PriceRow>(
    "SELECT id, amount FROM prices WHERE id = $1"
)
.bind(id)
.fetch_one(&pool)
.await?;

// ✅ 正确 — 编译期检查的查询（sqlx::query! 宏）
let price = sqlx::query_as!(
    PriceRow,
    "SELECT id, amount FROM prices WHERE id = $1",
    id
)
.fetch_one(&pool)
.await?;

// 必须运行 cargo sqlx prepare 并提交 .sqlx/ 目录
```

**检查方式：**
```bash
# CI 门禁 — 离线模式检查（验证 .sqlx/ 缓存存在且最新）
SQLX_OFFLINE=true cargo build 2>&1 | grep -q "error" && exit 1

# 检查 .sqlx/ 目录已提交
test -d .sqlx || exit 1
```

**执行：** CI build 步骤

## 规则与检查方式速查

| LINT | 类型 | 检查工具 | 阶段 |
|------|------|----------|------|
| LINT-001 | A | `clippy::disallowed_types` | 编译期 |
| LINT-002 | B | 测试期 AST 扫描 | `cargo test` |
| LINT-003 | C | `cargo deny check bans` | CI |
| LINT-004 | C | 脚本检查 Cargo.toml | CI |
| LINT-005 | A | `clippy::unwrap_used` + `expect_used` | 编译期 |
| LINT-006 | C | 脚本检查 Cargo.toml | CI |
| LINT-007 | D | ripgrep + AST 扫描 | Entropy GC |
| LINT-008 | A | `#![forbid(unsafe_code)]` | 编译期 |
| LINT-009 | C | ripgrep 检查 `#![allow]` | CI |
| LINT-010 | C | `SQLX_OFFLINE=true cargo build` | CI |

## 添加新规则流程

1. 发现 Agent 错误或 code review 反复出现的问题
2. 判断规则类型（A > B > C > D，优先编译期检查）
3. 编写规则定义 + 正反示例
4. 添加检查方式（clippy 配置 / 测试 / 脚本）
5. 在 CI 中集成（如适用）
6. 更新本文件
