# Rust Coding Skill

> Rust 编码实现技能。用于编码实现阶段，指导 Agent 按分层规范实现 Rust 代码。

## 输入

- `spec.md`
- `tasks.md`

## 输出

- 变更后的源码文件
- 更新的 `tasks.md`

## 执行前必须加载的分层 Spec

| 层级 | 规范文件 | 适用场景 |
|------|----------|----------|
| Handler | `rust-handler-spec.md` | 新增/修改 HTTP handler / axum 路由 |
| Service | `rust-service-spec.md` | 新增/修改业务服务（trait + impl） |
| Domain | `rust-domain-spec.md` | 新增/修改领域模型（Newtype / Entity） |
| Repository | `rust-repository-spec.md` | 新增/修改数据访问（sqlx 查询） |
| Client | `rust-client-spec.md` | 新增/修改外部服务调用（reqwest / gRPC） |

## 编码前置检查清单

- [ ] 已理解 `spec.md` 中的全部需求
- [ ] 已阅读 `tasks.md` 了解任务分解
- [ ] 已加载对应层级的分层编码 Spec
- [ ] 已读取需修改的现有代码文件
- [ ] 已确认修改不违反架构规则（`arch-rules-rust.md`）
- [ ] 已确认修改不违反编码规范（`coding-rules-rust.md`）
- [ ] 已运行 `cargo check` 确认当前代码可编译

## 通用编码原则

### 1. 最小变更原则

- 只改动需求要求的代码，不做无关联的优化
- 如果发现已有代码问题，记录在 `summary.md` 的"例外情况"中，不在此次修改
- 禁止顺手重构不相关的模块

### 2. 一致性原则

- 新代码的风格必须与代码库已有风格保持一致
- 使用代码库中已有的 Newtype、Error 类型、trait
- 禁止重新实现已存在的工具函数
- 使用 workspace 中已声明的依赖，禁止引入功能重复的 crate

### 3. 类型安全

- 禁止使用 `as` 进行可能丢失精度的类型转换（使用 `From`/`TryFrom`）
- 禁止使用 `unwrap()` / `expect()` — 使用 `?` 或显式模式匹配
- 禁止使用 `unsafe`（除非在专门的 unsafe 模块中且有 `// SAFETY:` 注释）
- 禁止返回裸 `String` / `i64` 代替业务 ID — 使用 Newtype
- 所有公共 API 返回 `Result<T, E>`，不 panic

### 4. 错误处理

- 库代码使用 `thiserror` 定义错误类型
- 应用代码使用 `anyhow::Result` + `.context()`
- 每个 `?` 传播点必须有 `.context()` 或 `.with_context()`
- 错误消息包含业务上下文（ID、操作名称）

### 5. 异步规范

- 所有 IO 操作使用 async
- handler 函数签名 `async fn` + `State<AppState>`
- 阻塞操作使用 `tokio::task::spawn_blocking`
- 禁止在 async 中使用 `std::thread::sleep`

## 编码后自检清单

- [ ] `cargo check` 通过（零错误）
- [ ] `cargo clippy -- -D warnings` 通过
- [ ] `cargo fmt --check` 通过
- [ ] 代码风格与代码库一致
- [ ] 所有新增公共函数有 `///` 文档注释
- [ ] 所有 `?` 传播点有 `.context()`
- [ ] 所有新增 trait 方法有对应的测试计划
- [ ] 没有使用 `unwrap()` / `expect()`
- [ ] 没有使用 `unsafe` 代码（或已有 `// SAFETY:` 注释）
- [ ] `cargo sqlx prepare` 已更新（如有数据库变更）
