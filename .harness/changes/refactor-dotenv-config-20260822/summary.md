# 变更摘要 — 统一 dotenv 环境变量配置加载

> 本变更的 Single Source of Truth。

## 基本信息
- **需求**：重构环境变量加载，统一由 dotenv 从 `.env` 注入（数据库/Redis/OKX/币安秘钥等），以点env 替代 config.toml 文件持久化
- **类型**：refactor
- **日期**：20260822
- **Owner**：Application Owner Agent

## 阶段执行状态
| 阶段 | 状态 | 备注 |
|------|------|------|
| 需求分析 | ✅ | design.md + spec.md + tasks.md |
| 编码实现 | ✅ | config 重构 + main 移除 config.toml |
| 单元测试 | ✅ | 新增 2 个 config 测试（默认中性/from_env 健壮性） |
| 单元测试 CI | ✅ | 本地等效验证 |

## 验证结果
- `cargo check --workspace --all-targets`：通过
- `cargo clippy --workspace --all-targets`：0 warning
- `cargo test --workspace`：**580 passed / 0 failed / 17 ignored**（+2 config）
- 前端未改动（纯 Rust/配置重构）

## 变更清单
| 文件 | 变更 | 说明 |
|------|------|------|
| `crates/common/src/config.rs` | 修改 | `Default` 中性化（OKX/币安不再读 env），新增 2 个单测 |
| `crates/common/src/config/from_env.rs` | 修改 | 新增 `OKX_*`/`BINANCE_*` 环境变量读取（唯一 env 入口） |
| `src-tauri/src/main.rs` | 修改 | `AppServices::new(infra)`（移除 `config.toml`/`config_path` 持久化） |
| `crates/repository/src/postgres.rs` | 修改 | 测试改用 `AppConfig::from_env().database`（去散落 dotenv） |
| `crates/data-layer/src/postgres.rs` | 修改 | 同上（去 `.unwrap()` 崩溃） |
| `.env.example` | 修改 | 补全 `from_env` 读取的全部覆盖项 |

## 关键决策
- **唯一 env 入口**：`AppConfig::from_env()` 读取全部变量；`default()` 为中性、确定性开发默认值（不读 env）。
- **dotenv**：仅 `main.rs` 启动时 `dotenv().ok()` 一次，把 `.env` 注入进程环境。
- **替代 config.toml**：`update_config` 仅内存更新（重启回退到 env），符合"dotenv 替代 config.toml"意图。
