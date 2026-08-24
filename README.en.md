# Quant Trading System

A professional quantitative trading system built with the **Rust + Tauri 2.0 + Vue3 + PostgreSQL + Redis** stack.

## Overview

A complete quant-trading solution covering data management, strategy development, backtest analysis, trade execution, risk control, and real-time monitoring.

### Core Features

- ✅ **Modular architecture**: layered across 13 Rust crates (high cohesion, low coupling)
- ✅ **Dual storage**: PostgreSQL (relational + RANGE-partitioned time series) + Redis (cache)
- ✅ **Full backtesting**: strategy dev, parameter optimization, performance evaluation (incl. **multi-symbol portfolio backtests** — split per symbol, aggregate once, no cross-symbol indicator contamination)
- ✅ **Smart execution algorithms**: TWAP, VWAP, Iceberg (sliced, risk-checked, executed via paper/live; time-proportional distribution + last-slice remainder absorption)
- ✅ **Three-tier risk control**: pre-trade (cash/position/daily-loss/concentration, sells use available qty consistently), in-trade, post-trade
- ✅ **Real-time monitoring**: Prometheus metrics + DB-cumulative order counts + equity-snapshot real values; trend charts with **dual axes** (order count left / amount right)
- ✅ **Order classification**: `orders.exchange` distinguishes **paper / live / algorithm**; trading page shows each type per tab (type badge, full trade-type labels)
- ✅ **Binance integration**: REST + WebSocket; live orders mirrored into `orders` + terminal-state reconciliation
- ✅ **Security hardening**: strong-secret fail-fast, enhanced key KDF, AES-GCM secret encryption, encrypted JWT-at-rest (backend key), login throttling, session `token_version` revocation, audit integrity
- ✅ **Type-safe errors**: `String` errors at the command layer; typed `ServiceError` in services
- ✅ **Modern UI**: Vue3 + Element Plus + ECharts + Pinia

## Architecture

```
quant-trading-system/
├── src-tauri/                      # Tauri backend (Rust)
│   └── src/
│       ├── main.rs                 # entry
│       ├── commands/               # Tauri commands (auth/audit/binance/backtest/...)
│       └── state.rs                # app state (session, services)
├── crates/                         # Rust workspace (13 crates + src-tauri)
│   ├── common/                     # shared types, config, utils
│   ├── domain/                     # domain layer (pure business logic, no IO)
│   ├── data-layer/                 # PostgreSQL + Redis + Binance data
│   ├── data-puller/                # background market/account snapshot puller
│   ├── repository/                 # repository (DB access abstraction)
│   ├── clients/                    # external clients (Redis, etc.)
│   ├── services/                   # service layer (business orchestration, typed errors)
│   ├── exchange-binance/           # Binance exchange integration
│   ├── strategy-layer/             # strategy dev + backtest + scheduler
│   ├── trading-layer/              # order execution + algorithms
│   ├── risk-layer/                 # risk control
│   ├── monitor-layer/              # monitoring + alerts
│   └── security/                   # crypto, auth, audit
├── src/                            # Vue3 frontend
│   ├── views/                      # pages (10 views)
│   ├── components/                 # common components
│   ├── composables/                # composables
│   ├── stores/                     # Pinia state
│   └── services/                   # API layer (incl. secureStorage adapter)
├── Cargo.toml                      # workspace config
├── package.json                    # frontend deps
└── .env.example                    # env template
```

## Quick Start

### Requirements

- **Rust** 1.77+, **Node.js** 18+, **PostgreSQL** 14+, **Redis** 6+

### Setup

```bash
git clone https://github.com/heheshang/quant_trading.git && cd quant_trading
cp .env.example .env
docker compose up -d postgres redis     # postgres → 127.0.0.1:15432, redis → 16379
npm install
cd src-tauri && DATABASE_HOST=127.0.0.1 DATABASE_PORT=15432 \
  DATABASE_USERNAME=quant DATABASE_PASSWORD=quant_password \
  DATABASE_NAME=quant_trading cargo run --bin migrate-db up
npm run tauri dev
```

> ⚠️ **Set strong random secrets (≥32 bytes)** or startup refuses (fail-fast):
> ```bash
> export JWT_SECRET=$(openssl rand -hex 32)
> export ENCRYPTION_KEY=$(openssl rand -hex 32)
> ```

## Modules (highlights)

- **common**: `AppConfig` (DB/Redis/trading/risk/Binance/security), types, utils.
- **data-layer**: `orders.exchange` column, `market_data` 2027 partitions, migration runner.
- **services**: `AccountService` (order counts, paper positions, equity snapshots), `AuthService` (token_version), `MarketService`, `BinanceService`, `RiskService`, `StrategyService`, `ConfigService`.
- **strategy-layer**: `Strategy` trait, indicators, `net_quantity` (net position sizing), no-lookahead backtest (signal at bar-t close, fill at bar-(t+1) open), per-symbol scheduler, multi-symbol portfolio backtest.
- **trading-layer**: `OrderManager`, paper scheduler (exchange-filtered, limit-crossing, timeout expiry), limit fill price clamped to limit, Binance executor.
- **risk-layer**: `PreTradeRiskChecker` (cash/position/daily-loss/concentration — sells use `available_quantity`, clamp ≥0), VaR.
- **monitor-layer**: Prometheus metrics (single-writer gauges), DB-cumulative metrics, dual-axis trend, background tasks (equity 60s / live-order 5s with change-detection + retry + backoff).
- **security**: AES-256-GCM (hex full-entropy decode + Argon2id KDF + legacy compat), JWT+token_version, login throttle, audit (success/failure), frontend `plugin-store` + backend-key encrypted JWT-at-rest.

## Testing

- Frontend: 24 files / 246 tests passing.
- Rust workspace: cargo test/clippy green; real PostgreSQL/Binance integration tests `#[ignore]` by default.
- Risk, backtest, multi-symbol aggregation covered.

## Security & Compliance

TLS transport; fail-fast on placeholder/short secrets; API secrets AES-GCM; Argon2 password hashing; JWT + token_version revocation; login throttling; encrypted session token at rest; audit logs (incl. order success/failure); live-order auth + pre-trade risk gated; `.env.example` has no secrets.

## Roadmap

- **Phase 1 (done)**: architecture, data layer, strategy/backtest, trading/algorithmic, risk, monitoring, security, Binance integration, UI, typed errors, full audit hardening (5 🔴 + 8 🟠 + 10 🟡 + C1/C2 + UI 21).
- **Phase 2 (in progress)**: migrations, WebSocket (reduced-frequency overview), optimizer; OS keychain (stronghold/keychain) pending.

## License

MIT. For study/research only. Trading involves risk; past performance does not guarantee future results.
