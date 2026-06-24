# Rust 架构规则

> 工程结构约束与分层架构约定。遵循 Rust 生态惯例。
> 核心原则：Cargo workspace 模块化、分层依赖单向、编译期约束优于运行时检查。

## 工作区结构

```text
my-service/                      # Cargo workspace root
├── Cargo.toml                   # [workspace] + [workspace.dependencies]
├── Cargo.lock                   # 提交到 VCS（应用项目）
├── rust-toolchain.toml          # 固定工具链版本
├── rustfmt.toml                 # 格式化配置
├── deny.toml                    # cargo-deny 许可证/依赖审计
├── .sqlx/                       # sqlx prepare 缓存（提交到 VCS）
│
├── crates/
│   ├── app/                     # 启动与路由装配
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs          # 入口点，最小化
│   │       ├── lib.rs           # 路由装配 + AppState 构建
│   │       ├── router.rs        # axum Router 定义
│   │       ├── state.rs         # AppState（共享依赖注入）
│   │       └── config.rs        # 配置加载（figment / config）
│   │
│   ├── handlers/                # HTTP Handler 层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── mod.rs
│   │       ├── price.rs         # price 相关 handler
│   │       └── extractors.rs    # 自定义 axum extractor
│   │
│   ├── services/                # 业务服务层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── mod.rs
│   │       └── price.rs         # PriceService trait + impl
│   │
│   ├── domain/                  # 领域模型层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── price.rs         # Price / PriceRule / Money
│   │       └── order.rs
│   │
│   ├── repository/              # 数据访问层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── mod.rs
│   │       ├── price_repo.rs    # PriceRepository trait + sqlx impl
│   │       └── models.rs        # 数据库行类型（Row / Entity）
│   │
│   ├── clients/                 # 外部服务调用层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── mod.rs
│   │       ├── external_price.rs
│   │       └── cache.rs         # Redis 缓存操作
│   │
│   └── common/                  # 公共层
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── errors.rs        # 统一错误类型
│           ├── newtypes.rs      # ID Newtype 定义
│           └── tracing.rs       # tracing 初始化
│
├── migrations/                  # sqlx 迁移脚本
│   └── 20260622000001_create_price_rules.sql
│
└── tests/                       # 集成测试
    └── price_api_test.rs
```

## 分层依赖规则

```
app → handlers → services → domain
  │        │          │
  │        │          └──→ repository
  │        │
  │        └──→ clients
  │
  └──→ common
```

- **上层可依赖下层，下层不可依赖上层**
- **domain 层零外部依赖**：不可依赖 repository / clients / handlers / app
- **common 可被任意层依赖，common 不可依赖任何业务层**
- **handlers 与 handlers 之间互不依赖**
- **services 之间可互相依赖（通过 trait）**

## Cargo.toml 依赖声明规范

```toml
# workspace Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "3"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
# Web framework
axum = "0.8"
# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "tls-rustls"] }
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# Error handling
thiserror = "2"
anyhow = "1"
# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
# Validation
validator = { version = "0.19", features = ["derive"] }
# Config
figment = { version = "0.10", features = ["toml", "env"] }
# Time
chrono = { version = "0.4", features = ["serde"] }
# UUID
uuid = { version = "1", features = ["v4", "v7", "serde"] }
# Testing
proptest = "1"
insta = "1"
tokio-test = "0.4"

[workspace.lints.clippy]
correctness = "deny"
suspicious = "deny"
style = "warn"
complexity = "warn"
perf = "warn"
unwrap_used = "deny"
expect_used = "deny"
panic_in_result_fn = "deny"
indexing_slicing = "deny"
string_slice = "deny"
missing_errors_doc = "warn"
missing_panics_doc = "warn"
missing_safety_doc = "deny"
```

```toml
# crates/handlers/Cargo.toml（示例：中间层 crate）
[package]
name = "my-service-handlers"
version.workspace = true
edition.workspace = true

[dependencies]
# workspace 依赖
axum.workspace = true
serde.workspace = true
tracing.workspace = true

# 内部依赖
my-service-services = { path = "../services" }
my-service-domain = { path = "../domain" }
my-service-common = { path = "../common" }

[lints]
workspace = true
```

## 核心架构原则

### 1. API 设计

- 所有公共 API（trait、struct）必须放在 crate 的 `lib.rs` 中 re-export
- 请求/响应 DTO 定义在 handlers crate 中，或独立 `dto` crate
- 使用 `axum::extract::State` 注入共享状态，禁止全局变量
- 响应统一使用 `Result<T, AppError>`，其中 `AppError` 实现 `IntoResponse`
- 所有 handler 函数必须是 `async fn`，接受 `State<AppState>` 和 extractor

### 2. 依赖注入

- 使用 `AppState` 结构体集中管理所有共享依赖
- trait 定义在对应的 crate 中，实现可以延迟到上层 crate（依赖反转）
- 禁止使用全局 OnceLock / lazy_static 作为依赖注入手段
- 测试时通过实现 trait 进行 Mock

```rust
// crates/app/src/state.rs
#[derive(Clone)]
pub struct AppState {
    pub price_service: Arc<dyn PriceService>,
    pub price_repo: Arc<dyn PriceRepository>,
    pub db_pool: PgPool,
    pub redis: Arc<RedisClient>,
}
```

### 3. 错误处理架构

- `common` crate 定义顶层 `AppError` enum（实现 `IntoResponse`）
- 各层定义自己的错误类型（`ServiceError`, `RepoError`, `ClientError`）
- 使用 `From` trait 实现错误类型之间的自动转换
- 错误必须包含可追踪的上下文信息

### 4. 数据库访问

- 使用 `sqlx` 作为唯一数据库驱动（编译期查询检查）
- 迁移脚本放在 `migrations/` 目录，命名格式 `YYYYMMDDHHMMSS_description.sql`
- 执行 `cargo sqlx prepare` 并将 `.sqlx/` 提交到 VCS（CI 离线检查）
- Repository trait 定义在 `domain` 或 `repository` crate 中
- 禁止在 handler / service 中直接写 SQL

### 5. 外部服务调用

- 所有外部调用封装在 `clients` crate 中
- 每个外部服务一个 module（`clients::external_price`）
- 必须实现 trait 以便 Mock 测试
- 必须设置超时和重试策略
- 使用 `tower` 中间件统一处理超时、重试、熔断

### 6. 配置管理

- 使用 `figment` 或 `config` crate 统一加载配置
- 配置来源优先级：环境变量 > 配置文件 > 默认值
- 敏感配置（密码、密钥）通过环境变量注入，禁止写入配置文件
- 配置结构体使用 `Deserialize` + 验证

### 7. 异步运行时

- 统一使用 `tokio` 作为异步运行时
- 禁止混用多个运行时（如 tokio + async-std）
- 阻塞操作（CPU 密集型）使用 `tokio::task::spawn_blocking`
- 禁止在 async 上下文中调用 `std::thread::sleep`

## 禁止的架构模式

| 模式 | 说明 | 替代方案 |
|------|------|----------|
| 循环依赖 | crate A 依赖 B，B 依赖 A | 提取公共 trait 到 common |
| 跨层直接调用 | handler 直接调用 repository | 通过 service 层 |
| domain 依赖基础设施 | domain 引入 sqlx / reqwest | trait 反转，实现在上层 |
| 全局可变状态 | `static mut` / `lazy_static!` 可变 | AppState + Arc |
| 裸类型穿透 | `String` 代替 `UserId` 在各层传递 | Newtype 包装 |
| 同步阻塞在 async 中 | `std::thread::sleep` / 同步 IO | spawn_blocking / async 替代 |
| `#[allow(...)]` 全局压制 | 在 lib.rs 中 `#![allow(clippy::xxx)]` | 行级 allow |
