# 变更摘要 — 接入币安交易所（后端基础）

> 本变更的 Single Source of Truth。

## 基本信息
- **需求**：新增币安（Binance）交易所支持
- **类型**：feat
- **日期**：20260822
- **本期范围**：后端基础（BN-1 crate + BN-2 配置）
- **Owner**：Application Owner Agent

## 阶段执行状态
| 阶段 | 状态 | 备注 |
|------|------|------|
| 需求分析 | ✅ | spec.md + design.md + tasks.md |
| 编码实现 | ✅ | BN-1 + BN-2 |
| 单元测试 | ✅ | `exchange-binance` 9 用例 |
| 单元测试 CI | ✅ | 本地等效验证 |
| 后续增量 | ⬜ | BN-3装配 / BN-4 WS / BN-5服务命令 / BN-6前端 |

## 验证结果
- `cargo check --workspace`：通过
- `cargo clippy --workspace --all-targets`：0 warning
- `cargo test --workspace`：**568 passed / 0 failed / 17 ignored**（+9 Binance）
- `exchange-binance --features test-utils`：编译通过

## 变更文件清单
| 文件 | 变更 | 说明 |
|------|------|------|
| `crates/exchange-binance/Cargo.toml` | 新增 | workspace 依赖 + `test-utils` feature |
| `crates/exchange-binance/src/lib.rs` | 新增 | re-export + mock gate |
| `crates/exchange-binance/src/types.rs` | 新增 | `BinanceEnvironment`、DTO、symbol 互转 |
| `crates/exchange-binance/src/client.rs` | 新增 | `ClientInterface` + `Client`（reqwest + HMAC-SHA256） |
| `crates/exchange-binance/src/mock_data/mod.rs` | 新增 | 测试数据（test-utils） |
| `crates/exchange-binance/src/tests.rs` | 新增 | 签名/kline/错误/符号单测 |
| `crates/common/src/config.rs` | 修改 | `BinanceConfig` + `AppConfig.binance`（BINANCE_* 环境变量） |
| `crates/common/src/lib.rs` | 修改 | 导出 `BinanceConfig` |
| `Cargo.toml` | 修改 | workspace members + `hmac/sha2/hex` 依赖 |

## 后续增量（design.md §5）
- **BN-3** 装配注入 `AppServices`/`AppState` 客户端
- **BN-4** Binance WebSocket（镜像 `OkxWebSocket`）
- **BN-5** `BinanceService` + `commands/binance.rs`
- **BN-6** 前端 `services/binance*.ts` + UI 切换

## 已知要点
- Binance spot/futures REST 基址与 HMAC-SHA256 签名；`symbol` 格式（`BTCUSDT`）与 app 领域（`BTC-USDT`）互转。
- 本期无实盘 on-chain 调用，仅 REST 端点 + 单测；后续接入需在 `AppState`/命令层注入。
