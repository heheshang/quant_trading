# 需求规格说明书 — 接入币安交易所（后端基础）

## 背景

系统目前仅支持 OKX。需新增 Binance 支持。Binance 无官方 Rust SDK，基于 `reqwest` + HMAC-SHA256 直连 REST。本变更落地**后端基础 crate**（BN-1）+ 配置（BN-2），为 websocket/服务/命令/前端增量打底。

## 需求描述

1. 新增 `crates/exchange-binance` crate（镜像 `exchange-okx` 结构）：
   - `types.rs`：Binance DTO + 映射到 `quant_common` 领域类型（`MarketData`/`Account`/`Order`/`OrderBook`）。
   - `client.rs`：`ClientInterface`（`async_trait` + `mockall::automock` 带 `test-utils`）与 `Client`（`reqwest` + HMAC 签名）。
   - 核心方法：`get_candles` / `get_account_balance` / `get_order_book` / `place_order` / `cancel_order`。
   - `mock_data`（`test-utils`）。
2. `common/config.rs` 增加 `BinanceConfig`（`BINANCE_*` 环境变量），并入 `AppConfig`。

## 变更范围

### 涉及模块
- `crates/exchange-binance`（新增 crate）
- `crates/common/src/config.rs`（新增 `BinanceConfig`）
- `Cargo.toml`（workspace members + workspace.dependencies）
- `crates/common/src/lib.rs`（导出 `BinanceConfig`）

### 涉及代码层
- [x] Client / RPC（新增交易所 client）
- [x] Service / 用例层（配置准备，服务暂后续）
- [x] Configuration（`BinanceConfig`）
- [ ] Migrations（无）

### 不涉及
- 数据库 schema / 迁移（无变更）
- 现有 OKX 逻辑（不改行为）
- 前端 UI（本期不做，见 design.md §5）

## 影响分析

| 维度 | 分析 | 风险 |
|------|------|------|
| 上游兼容 | 新增独立 crate，与 OKX 解耦 | 低 |
| 数据兼容 | 无 schema 变化 | 低 |
| 配置 | `AppConfig` 增加 `binance` 段（默认禁用） | 低 |
| 性能 | 无热点影响 | 低 |
| 安全 | 签名基于密钥；不做实盘 on-chain 调用 | 低 |

## 验收标准

1. `cargo check --workspace` 通过。
2. `cargo test --workspace` 通过；`exchange-binance` 单测（HMAC 签名正确性、kline 解析、错误映射、符号互转）通过。
3. `cargo clippy --all-targets` 0 warning。
4. `AppConfig` 反序列化含 `binance` 段；`binance.enable` 默认 `false`。

## 备注

- Binance spot 基址 `https://api.binance.com`；HTTPS 用 `reqwest` rustls。
- 新增依赖：`hmac`、`sha2`、`hex`（工作区）。
