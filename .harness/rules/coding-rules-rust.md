# Rust 编码规范

> 编码风格、命名规范、类型约束。每一条规则对应一个历史失败案例或 Rust 社区共识。
> 规则遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) 和 [Rust Style Guide](https://doc.rust-lang.org/stable/style-guide/)。

## 类型与单位约束

| 约束 | 规范 | 反例 | 原因 |
|------|------|------|------|
| 金额/价格 | `i64` 或 `u64` 类型，单位为最小单位（分/satoshis） | `f64` / `f32` | 浮点数精度丢失，IEEE 754 无法精确表示十进制金额 |
| 金额计算 | 整数运算或 `rust_decimal::Decimal` | `f64` 运算 | 避免浮点舍入误差 |
| 时间字段 | `chrono::NaiveDateTime` / `time::OffsetDateTime` | `String` 或 `i64` 裸时间戳 | 类型安全，避免时区混淆 |
| 百分比 | `u32` 万分比（basis points，0-10000） | `f64` 百分比 | 精度损失，比较不可靠 |
| ID 字段 | 使用 Newtype 包装（`UserId(uuid::Uuid)`） | `String` / `i64` 裸类型 | 编译期类型区分，防止误用 |
| 状态字段 | `enum` 而非 `String` 或 `bool` | `String` 或多个 `bool` | 让非法状态不可表示 |
| 可选值 | `Option<T>` | `""` 或 `-1` 哨兵值 | 类型系统表达语义，避免魔法值 |
| 字节大小 | `u64`（字节） | `f64` | 整数精确，避免浮点比较 |

## 命名规范

### Rust 命名（遵循 RFC 430）

- **类型名**（struct/enum/union/trait/type alias）：`UpperCamelCase`（`PriceService`, `OrderQuery`, `ConnectionPool`）
- **函数/方法名**：`snake_case`（`get_price`, `calculate_discount`）
- **常量/静态变量**：`SCREAMING_SNAKE_CASE`（`MAX_RETRY_COUNT`, `DEFAULT_TIMEOUT`）
- **变量/参数**：`snake_case`（`item_id`, `price_map`）
- **模块/包名**：`snake_case`（`price_engine`, `order_service`）
- **Cargo 包名**：`kebab-case`（`price-engine`, `order-service`）
- **Feature 名**：`kebab-case`（`enable-tracing`, `use-serde`）
- **生命周期**：短字母，优先 `'a`，有语义时用 `'ctx` / `'req` 等
- **宏名**：`snake_case!` 用于函数式宏，`CamelCase!` 用于 derive 宏
- **构造器**：`new()` 无参数，`with_*()` 有参数，`from_*()` 从其他类型转换

### 布尔变量
- 禁止 `is_` / `has_` / `can_` 以外的前缀用于布尔值
- 布尔参数在公共 API 中**必须替换为 enum**（C-CUSTOM-TYPE）

### 数据库命名
- **表名**：小写蛇形，业务前缀
- **字段名**：小写蛇形
- **主键**：`id`
- **索引名**：`idx_{表名}_{字段名}`

## 代码规范

### 函数设计
- 函数体不超过 80 行，超出考虑提取子函数
- 函数参数不超过 4 个，超出用 struct 封装或 Builder 模式
- 公共函数必须标注 `#[must_use]`（如果返回值不应被忽略）
- 纯函数优先：相同输入总是相同输出，无副作用

### 模块组织
- 使用 2018 edition 风格：`src/foo.rs` + `src/foo/bar.rs`，**不使用** `src/foo/mod.rs`
- `lib.rs` 声明公共 API，`main.rs` 仅做入口和初始化
- 模块可见性默认 `pub(crate)`，只在必要时 `pub`
- 使用 `use` 而非完整路径，避免过度 `use super::*`

### 错误处理（详见 rust-error-handling-spec.md）
- **库代码**：使用 `thiserror` 派生自定义错误类型，**禁止**返回 `anyhow::Result`
- **应用代码**：使用 `anyhow::Result` + `.context()` 添加上下文
- **禁止**在生产代码中使用 `unwrap()` 或 `expect()`
- 每个 `?` 传播点必须添加 `.context()` 或 `.with_context()`
- 错误类型必须实现 `std::error::Error` + `Send + Sync` + `'static`
- 错误消息必须包含业务上下文（如 order_id, user_id）

### 日志与追踪（使用 `tracing` crate）
- 统一使用 `tracing` 生态（`tracing` + `tracing-subscriber`）
- 日志级别：
  - `ERROR`：系统异常，需要人工介入
  - `WARN`：业务异常，可自动恢复
  - `INFO`：关键业务流程节点
  - `DEBUG`：调试信息
- 异步函数必须使用 `#[tracing::instrument]` 自动创建 span
- 关键路径必须记录结构化字段（`info!(order_id = %id, "processing")`）
- 禁止在循环中打印日志
- 敏感信息（密码、手机号、身份证）必须脱敏

### 注释规范
- 公共类型/函数必须有 `///` 文档注释，包含 Examples
- 模块级文档（`//!`）描述模块职责和使用场景
- 复杂业务逻辑必须有行内注释说明意图
- **禁止注释掉的代码**（直接删除，git 历史可恢复）
- 每个 `unsafe` 块必须有 `// SAFETY:` 注释说明安全前提

### unsafe 代码
- `unsafe` 代码**必须**有 `// SAFETY:` 注释
- 优先使用 `#![forbid(unsafe_code)]`（对不包含 unsafe 的 crate）
- 所有 unsafe 函数必须记录安全前置条件
- CI 中运行 `cargo +nightly miri test` 验证 unsafe 代码

## 文件规范

- 单文件不超过 500 行（不含自动生成的代码）
- 一个文件一个主要类型（struct/enum/trait）
- 关联类型和辅助函数放在同一文件中
- 测试模块 `#[cfg(test)] mod tests` 放在文件末尾
- 禁止 `#[allow(...)]` 全局压制 lint，使用行级 `#[allow(clippy::xxx)]`

## Clippy 配置（强制）

### 工作区级别（Cargo 1.74+，`workspace Cargo.toml`）

```toml
[workspace.lints.clippy]
# 核心组 — deny 级别，阻塞 CI
correctness = "deny"
suspicious = "deny"

# 质量组 — warn 级别
style = "warn"
complexity = "warn"
perf = "warn"

# 生产安全 — deny 级别
unwrap_used = "deny"
expect_used = "deny"
panic_in_result_fn = "deny"
indexing_slicing = "deny"
string_slice = "deny"

# 公共 API 质量
missing_errors_doc = "warn"
missing_panics_doc = "warn"
missing_safety_doc = "deny"
missing_docs_in_private_items = "allow"

# 常见例外（团队评审后开放）
module_name_repetitions = "allow"
must_use_candidate = "allow"
too_many_arguments = "allow"

# 每个成员 crate 必须 opt-in
```

```toml
# 成员 crate Cargo.toml
[lints]
workspace = true
```

### CI 执行

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

## 格式化配置

```toml
# rustfmt.toml
style_edition = "2024"
max_width = 100
group_imports = "StdExternalCrate"
imports_granularity = "Module"
use_small_heuristics = "Max"
```

## 测试规范

- 单元测试覆盖率：新增代码 ≥ 80%
- 测试函数命名：`test_{函数名}_{场景}_{预期结果}`
- 每个测试函数包含 Arrange-Act-Assert 三段式结构
- 单元测试放在 `#[cfg(test)] mod tests` 中
- 集成测试放在 `tests/` 目录
- 禁止测试依赖外部环境（DB/缓存/RPC 需 Mock/TestContainer）
- 纯函数优先使用 `proptest` 进行属性测试
- 复杂输出使用 `insta` 进行快照测试
- 测试数据构建使用 `fake` crate 或手动 Builder

## 依赖管理

- 工作区依赖统一在 `[workspace.dependencies]` 中声明
- 每个 crate 在 `[dependencies]` 中使用 `workspace = true` 引用
- 指定 `default-features = false` 显式控制 feature
- 运行 `cargo audit --deny warnings` 检查已知漏洞
- 运行 `cargo deny check` 检查许可证和重复依赖
- 禁止使用 `*` 版本号，使用精确版本或 `^` 语义化版本

## Cargo.toml 规范

```toml
[package]
name = "my-crate"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"
description = "简要描述 crate 功能"
repository = "https://github.com/org/repo"
readme = "README.md"
keywords = ["category1", "category2"]
categories = ["category::subcategory"]

[dependencies]
# workspace 依赖
tokio = { workspace = true }
serde = { workspace = true }

# 直接依赖（非 workspace）
thiserror = "2"

[dev-dependencies]
proptest = "1"
insta = "1"
tokio-test = "0.4"

[lints]
workspace = true
```
