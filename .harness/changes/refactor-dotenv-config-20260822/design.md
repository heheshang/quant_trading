# 设计蓝图：统一 dotenv 环境变量配置加载

> 目标：把配置来源统一为由 **dotenv** 从 `.env` 注入，覆盖数据库连接、Redis 连接、OKX/币安秘钥等全部凭据；拆除散落的 `std::env::var`/`dotenv::var` 读取与 `config.toml` 文件持久化。

## 1. 现状问题（AS-IS）

| 问题 | 位置 |
|------|------|
| `dotenv` / `std::env` 读取混用 | `main.rs` 用 `dotenv`；`config.rs` `Default` 用 `std::env`；`from_env.rs` 用 `env_string` 封装 |
| 环境读取**分散** | `Default` impl 读 OKX/币安 env；`from_env` 读其余 env —— 两处割裂 |
| 硬编码默认密钥 | `Default` 内 `jwt_secret: "change_this_secret..."`、DB 密码 `"quant_password"` |
| `config.toml` 文件持久化 | `ConfigService::with_path` 将运行时配置写盘；与 dotenv 并存、语义重叠 |
| 散落 `dotenv::var` + `.unwrap()` | `repository/postgres.rs`、`data-layer/postgres.rs` 的 `#[cfg(test)]`；`data-layer/postgres.rs` 用 `.unwrap()` 崩溃 |

## 2. 目标（TO-BE）

```
启动（main.rs）
  └─ dotenv::dotenv().ok()          # 仅此一处加载 .env
  └─ AppConfig::from_env()          # 唯一 env 构建入口（读全部变量）
       └─ 基于 AppConfig::default()（中性确定性默认，不读 env）
            └─ 逐一覆盖 DB/Redis/OKX/Binance/安全/交易/风控/监控/调度/优化器
```

- **`AppConfig::default()`**：中性、确定性开发默认值（**不读任何环境变量**）。
- **`AppConfig::from_env()`**：唯一读取环境变量的入口（DB/Redis/OKX/币安/安全/…）。
- **`dotenv`**：仅 `main.rs` 启动时调用一次，把 `.env` 注入进程环境；各层通过构造注入的 `AppConfig` 取配置，**不再散落读 env**。
- **移除 `config.toml` 持久化**：运行时配置以 env 为准；`ConfigService::with_path` 保留 API 但 `main.rs` 不再传入路径（用 `AppServices::new`）。

## 3. 变更范围

| 模块 | 变更 |
|------|------|
| `crates/common/src/config.rs` | `Default` 中性化（去掉 OKX/Binance 环境的 `std::env::var`）；保留 `env_*` helper |
| `crates/common/src/config/from_env.rs` | 新增 OKX + Binance 的环境变量读取；确保覆盖全部既有变量 |
| `src-tauri/src/main.rs` | 启动仅 `dotenv().ok()`；改用 `AppServices::new(infra)`（去 `config_path`） |
| `crates/repository` / `data-layer` `postgres.rs` | 测试模块改用统一的 `DatabaseConfig` 构建（去散落 `dotenv::var`/`.unwrap()`） |
| `.env.example` | 完整列出全部变量（DATABASE_*/REDIS_*/OKX_*/BINANCE_*/SECURITY_* 等） |
| `crates/services/src/config_service.rs` | `with_path` 保留但标注"不再被 main 使用"；`update_config` 走内存 |

## 4. 验收标准

1. `cargo check --workspace --all-targets` 通过。
2. `cargo clippy --all-targets` 0 warning。
3. `cargo test --workspace` 通过（无退出）。
4. `AppConfig::default()` 不读环境变量（单元测试：设置 `OKX_API_KEY=xyz` 后 `default().okx.api_key == ""`）。
5. `AppConfig::from_env()` 能读取 DB/Redis/OKX/Binance 变量（单元测试）。
6. `main.rs` 不再出现 `config.toml`/`config_path`。

## 5. 风险与回退

- **行为变化**：`update_config` 不再写盘（重启回退到 env）。符合"dotenv 替代 config.toml"意图；若需保留持久化可再启用 `with_path`。
- **无 panic**：全部 env 读取带默认/安全解析。
- **回退**：改动集中在 `config.rs`/`from_env.rs`/`main.rs`，可独立回滚。
