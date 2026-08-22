# 任务分解 — 接入币安（后端基础 BN-1/B N-2）

## 里程碑

| 任务 | 目标 | 验证 |
|------|------|------|
| BN-1 | 新增 `exchange-binance` crate（types + client + mock + 单测） | `cargo check` + `cargo test` |
| BN-2 | `BinanceConfig` + workspace 依赖 + `AppConfig` 段 | `cargo check` |

## 任务清单

### BN-1 crate
- [x] BN-1-1 `crates/exchange-binance/Cargo.toml`（依赖 common/security + reqwest/hmac/sha2/hex；`test-utils` feature）
- [x] BN-1-2 `types.rs`：`BinanceEnvironment`、DTO（Kline/Account/Order/OrderBook）、`symbol` 互转工具
- [x] BN-1-3 `client.rs`：`ClientInterface` trait + `Client`（reqwest + HMAC 签名 + 端点映射）
- [x] BN-1-4 `lib.rs`：re-export + `#[cfg] mock_data` + `MockBinanceClient`
- [x] BN-1-5 `tests.rs`：HMAC 签名、kline 解析、错误映射、符号互转单测
- [x] BN-1-6 `mock_data/`：测试数据（feature=test-utils）
- [x] BN-1-7 `cargo check` + `cargo test` 通过

### BN-2 配置
- [x] BN-2-1 `common/config.rs`：`BinanceConfig` + `AppConfig.binance` + 默认值 + 环境变量读取
- [x] BN-2-2 `common/lib.rs`：导出 `BinanceConfig`
- [x] BN-2-3 `Cargo.toml`：workspace members 加入 `crates/exchange-binance`；workspace.dependencies 加 `hmac/sha2/hex`
- [x] BN-2-4 `cargo check` + `cargo test` 通过

### 后续（本期不实现，见 design.md §5）
## 已完成增量（BN-3 + BN-5）

- [x] BN-3 装配注入：`SharedInfra`/`AppServices`/`AppState` 注入 `binance_client`；`main.rs` 构建 Binance client
- [x] BN-5 服务与命令：新增 `services/binance_service.rs`（`BinanceService`）+ `commands/binance.rs`（6 命令）并注册
- [x] 单测：`BinanceService` 4 用例（Mock）；workspace 572 passed

- [x] BN-6 前端：`services/binance.ts` + `binanceOrder.ts`（经 `transport.call`）+ `views/Binance.vue`（状态/余额/下单）+ 路由/侧边栏 + `types.ts`（Binance 类型 + `AppConfig.binance`）+ `binance.test.ts`（5 用例）
- [x] 验证：vue-tsc ✅；npm test 35 文件 / 431 passed

- [x] BN-4 WebSocket 客户端：`exchange-binance/src/websocket.rs`（`BinanceWebSocket`：订阅追踪 + 连接/自动重连 + `get_receiver` + kline/depth 消息解析 + 4 单测）
- [x] 验证：clippy 0 warning；workspace 576 passed（+4 ws）

> BN-4 已完成 **WS 客户端基础**；将其实时接入 `ws_commands.rs` + 前端（live streaming）可作为后续扩展（REST 已覆盖功能主路径）。
