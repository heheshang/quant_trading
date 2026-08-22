# 需求规格说明书 — 统一 dotenv 环境变量配置加载

## 背景
应用配置目前散落多处：`config.rs::Default` 读 OKX/币安 env、`from_env.rs` 读其余 env、`main.rs` 用 dotenv、`config.toml` 文件持久化并存，且部分模块用 `dotenv::var`/`.unwrap()` 崩溃。需统一为 **dotenv 从 `.env` 注入全部配置**（数据库/Redis/OKX/币安秘钥等）。

## 需求描述
1. `AppConfig::default()` 中性化：确定性开发默认值，**不读任何环境变量**。
2. `AppConfig::from_env()` 作为唯一 env 构建入口：读取 DB/Redis/OKX/Binance/安全/交易/风控/监控等全部变量。
3. `main.rs` 启动仅 `dotenv().ok()` 一次 + `AppConfig::from_env()`。
4. 移除 `config.toml` 文件持久化（运行时配置以 env 为准）。
5. 清理非生产代码中的散落 `dotenv::var`/`.unwrap()`。

## 变更范围
- [x] Configuration（`config.rs` / `from_env.rs`）
- [x] Application（`main.rs` 装配）
- [x] Repository / Data-layer（测试去散落）
- [ ] Migrations（无）

## 影响分析
| 维度 | 分析 | 风险 |
|------|------|------|
| 配置 | `default()` 不再读 env；`from_env()` 覆盖全部 | 低 |
| 行为 | `update_config` 不再写盘（重启回退 env） | 低（符合意图） |
| 兼容 | `ConfigService::with_path` 保留 API（暂未使用） | 低 |
| 性能/安全 | 无热点；密钥均来自 env（dotenv） | 低 |

## 验收标准
1. `cargo check --workspace --all-targets` 通过。
2. `cargo clippy --all-targets` 0 warning。
3. `cargo test --workspace` 通过。
4. `default().okx.api_key == ""`；`from_env()` 读取变量且不 panic。
5. `main.rs` 无 `config.toml`/`config_path`。
