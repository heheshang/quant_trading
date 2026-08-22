# Binance 接入 — PR / 发布说明

> 币安交易所全链路接入完成。以下内容可直接粘贴到 GitHub PR 描述框 / 追加到 `CHANGELOG.md`。

## PR 标题
`feat(binance): add full Binance exchange integration (REST + WebSocket + UI)`

## 概要
新增与 OKX 对称的币安（Binance）交易所支持，覆盖后端客户端、WebSocket 实时流、服务/命令层、配置与前端 UI。与 OKX 完全解耦，`BINANCE_*` 配置启用。

## 改动要点

### 后端（Rust）
- **`crates/exchange-binance`**：`reqwest` + HMAC-SHA256 直连 REST（spot/futures），含 `BinanceEnvironment`、DTO、符号互转、`MockBinanceClient`；`BinanceWebSocket`（订阅追踪、自动重连、kline/depth 解析）。
- **服务与命令**：`BinanceService`（镜像 `OkxService`）+ `commands/binance.rs`（余额/ K线/深度/下单/撤单/状态）+ `commands/binance_ws.rs`（实时流）。
- **配置/装配**：`BinanceConfig`（`BINANCE_API_KEY/SECRET/ENVIRONMENT/ENABLE`）并入 `AppConfig`；`SharedInfra`/`AppServices`/`AppState` 注入客户端与 WS 状态。

### 前端（Vue/TS）
- **services**：`binance.ts`（余额/K线/深度/状态 + WS 流函数）与 `binanceOrder.ts`（下单/撤单），经统一 `call()` 传输。
- **视图**：`views/Binance.vue` — 连接状态、余额表、REST 下单、实时行情面板（开始/停止 + 实时 K线）；路由 `/binance` + 侧边栏「币安交易」。
- **类型**：`Binance*` 类型 + `AppConfig.binance`。

## 测试
| 检查 | 结果 |
|------|------|
| `cargo test --workspace` | ✅ 578 passed / 0 failed / 17 ignored |
| `cargo clippy --all-targets` | ✅ 0 warning |
| `vue-tsc --noEmit` | ✅ |
| `npm test` | ✅ 35 文件 / 436 passed |

## 启用方式
在 `.env`/环境变量配置：
```
BINANCE_API_KEY=...
BINANCE_API_SECRET=...
BINANCE_ENVIRONMENT=spot    # spot 或 futures
BINANCE_ENABLE=true
```

## 说明
- 与 OKX 独立，互不影响；无数据库 schema / 迁移变更。
- 实时行情走 Binance WebSocket（`wss://stream.binance.com:9443`），REST 覆盖功能主路径。
