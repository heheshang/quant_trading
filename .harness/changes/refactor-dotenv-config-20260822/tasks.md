# 任务分解 — 统一 dotenv 环境变量配置加载

## 任务清单
- [x] 任务1 `config.rs::Default` 中性化：OKX/币安不再读 env（确定性默认）
- [x] 任务2 `from_env.rs`：新增 `OKX_*`/`BINANCE_*` 读取（唯一 env 入口，覆盖 DB/Redis/交易/风控/监控/安全/调度/优化器）
- [x] 任务3 `main.rs`：改用 `AppServices::new(infra)`，移除 `config.toml`/`config_path`
- [x] 任务4 `repository`/`data-layer` 测试：改为 `AppConfig::from_env().database`（去散落 dotenv/`unwrap`）
- [x] 任务5 `.env.example`：补全 `from_env` 读取的全部变量
- [x] 任务6 新增 config 单测（默认中性 + from_env 健壮性）
- [x] 任务7 验证：cargo check / clippy 0 / test 580 passed

## 验证命令
```
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets
cargo test --workspace --no-fail-fast
```
