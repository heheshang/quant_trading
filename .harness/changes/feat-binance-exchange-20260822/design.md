# 设计蓝图：接入币安（Binance）交易所

> 目标：在保留 OKX 现有集成的前提下，新增 Binance 交易所支持。
> 采用与 `exchange-okx` 对称的 **crate-first** 架构；本变更落地**后端基础 crate**（types + REST client + mock + 单测 + 配置），websocket/服务/命令/前端作为**后续增量**（见 §5）。

## 1. 背景与定位

现有系统仅接 OKX（`crates/exchange-okx`，基于官方 `okx` SDK，约 2741 行）。用户希望新增菜安（币安 Binance）支持。Binance 无官方 Rust SDK，需基于 `reqwest` + HMAC-SHA256 签名直连 REST。

**工程约束**：不破坏现有 OKX；新增交易所通过**对称 crate**接入，`AppServices`/命令层增加可选分支，配置增加 `binance` 段。

## 2. Binance REST 关键事实

| 项 | 值 |
|----|----|
| SPOT REST 基址 | `https://api.binance.com` |
| FUTURES-USDT REST 基址 | `https://fapi.binance.com` |
| WebSocket 基址 | `wss://stream.binance.com:9443` |
| 公开行情 | 无需鉴权 |
| 私有账户/下单 | Header `X-MBX-APIKEY` + 查询串 `signature=HMAC_SHA256(query, secret)` |
| symbol 格式 | 与 OKX 不同：`BTCUSDT`（OKX 为 `BTC-USDT`）|
| klines 间隔 | `1m/5m/15m/1h/4h/1d...` |

## 3. 模块结构（crate-first，镜像 exchange-okx）

```
crates/exchange-binance/
├── Cargo.toml                 # quant-common + security + reqwest + hmac/sha2
└── src/
    ├── lib.rs                 # re-export Client/ClientInterface/BinanceTypes
    ├── types.rs               # Binance 响应 DTO + 映射到 quant_common 领域类型
    ├── client.rs              # Client + ClientInterface（REST + HMAC 签名）
    ├── mock_data/             # 测试/演示数据（feature=test-utils）
    └── tests.rs               # 客户端单测
```

### 3.1 关键类型（`types.rs`）
- `BinanceEnvironment`（`Spot`/`Futures`）
- 响应 DTO（`Kline`, `AccountBalance`, `Order`, `OrderBookL2`…）→ 映射 `quant_common::types::{MarketData, Account, Position, Order, OrderBook}`
- `BinanceSymbol` 工具：`OKX_BTC-USDT` ⇄ `BINANCE_BTCUSDT` 互转（与 `MarketData.symbol` 兼容）

### 3.2 客户端（`client.rs`）
- `ClientInterface`（`#[async_trait]` + `mockall::automock`，带 `test-utils` feature）——与 OKX 对称
- `Client::new(api_key, api_secret, environment)` 构建 `reqwest::Client`
- `sign_query(query, secret) -> String` 用 `hmac-sha256` 生成签名
- 方法（本期）：
  - `get_candles(symbol, interval, limit) -> Vec<MarketData>`（GET `/api/v3/klines`）
  - `get_account_balance() -> Vec<Account>`（GET `/api/v3/account`，私有签名）
  - `get_order_book(symbol, limit) -> OrderBook`（GET `/api/v3/depth`）
  - `place_order(req) -> Order` / `cancel_order(symbol, order_id)`（私有签名）
- 依赖注入：新增 `security::hmac` 或复用 `required` 加密；本期用 `hmac` + `sha2` crate

### 3.3 配置（`common/config.rs`）
```rust
pub struct BinanceConfig {
    pub api_key: String,
    pub api_secret: String,
    pub environment: String, // "spot" / "futures"
    pub enable: bool,
}
```
`AppConfig` 增加 `pub binance: BinanceConfig`（env 前缀 `BINANCE_*`）。

## 4. 依赖注入与集成点

- `crates/services`：新增 `BinanceService`（镜像 `OkxService`），经 `AppServices` 装配；`OkxService`/`BinanceService` 共享 `MarketDataProvider` 抽象以支持多交易所。
- `src-tauri`：`AppState` 增加可选 `binance_client`；新增 `commands/binance.rs`。
- `data-layer::OkxDataSource`：抽象为通用 `ExchangeDataSource` 或新增 `BinanceDataSource`。
- 前端：`services/binance.ts`、`services/binanceOrder.ts`（镜像 okx 拆分）；UI 增加交易所切换。

## 5. 变更批次与范围（本期 vs 后续）

| 批 | 范围 | 本期 |
|----|------|------|
| **BN-1 后端 crate** | `exchange-binance`：types + REST client + mock + 单测 | ✅ 本期 |
| **BN-2 配置** | `BinanceConfig` + `AppConfig` 段 + 环境变量 | ✅ 本期 |
| **BN-3 装配** | workspace + `AppServices`/`AppState` 注入 client | ⬜ 后续（需 src-tauri 装配） |
| **BN-4 WebSocket** | Binance WS 订阅（镜像 `OkxWebSocket`） | ⬜ 后续 |
| **BN-5 服务/命令** | `BinanceService` + `commands/binance.rs` | ⬜ 后续 |
| **BN-6 前端** | `services/binance*.ts` + 组件/切换 | ⬜ 后续 |

> 本期交付 **BN-1 + BN-2**（后端基础，可编译、可测试），为后续增量打底。**不做** schema/迁移变更。

## 6. 验收标准（本期）

1. `cargo check --workspace` 通过。
2. `cargo test --workspace` 通过；`exchange-binance` 客户端单测（签名/解析/错误映射）通过。
3. `cargo clippy --all-targets` 0 warning。
4. `AppConfig` 反序列化含 `binance` 段；`binance` 字段默认值为空/禁用。

## 7. 风险与回退

- **API 精度风险**：Binance 返回字段/签名格式与 OKX 不同，映射需谨慎；以 `/api/v3/*` 文档为准，前端展示暂不依赖。
- **无真实密钥**：本期不含 on-chain 实盘调用，仅端到端（mock / 本地）验证。
- **回退**：`exchange-binance` 与 OKX 完全解耦，失败不影响现有功能；可独立回滚 crate。
