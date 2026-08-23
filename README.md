# 量化交易系统

一个专业的量化交易软件系统，基于 **Rust + Tauri 2.0 + Vue3 + PostgreSQL + Redis** 技术栈构建。

## 📋 项目概述

本系统是一个完整的量化交易解决方案，涵盖数据管理、策略开发、回测分析、交易执行、风险管理和实时监控等核心功能。

### 核心特性

- ✅ **模块化架构**：13 个 Rust crate 分层设计，高内聚低耦合
- ✅ **双数据库存储**：PostgreSQL（关系型 + RANGE 分区时序）+ Redis（缓存）
- ✅ **完整回测系统**：支持策略开发、参数优化、性能评估
- ✅ **智能执行算法**：TWAP、VWAP、冰山订单等（`run_algorithmic_order` 已接线：拆分后按普通市价/限价子单走风控+纸面/实盘执行）
- ✅ **三层风控体系**：事前、事中、事后全流程风险管理
- ✅ **实时监控告警**：Prometheus 指标 + 多渠道告警
- ✅ **Binance 交易所集成**：REST + WebSocket 对接，支持现货/合约（`BINANCE_ENVIRONMENT=spot` 或 `futures`）
- ✅ **类型安全错误处理**：命令层以 `String` 错误返回，服务层 typed errors
- ✅ **现代化界面**：Vue3 + Element Plus + ECharts + Pinia

## 🏗️ 系统架构

```
quant-trading-system/
├── src-tauri/                      # Tauri 后端入口（Rust）
│   └── src/
│       ├── main.rs                 # 主入口
│       ├── commands/               # Tauri 命令层（多文件：twofa/audit/optimizer 等）
│       ├── ws_commands.rs          # WebSocket 命令
│       └── state.rs                # 应用状态
├── crates/                         # Rust 工作空间（13 个业务 crates + src-tauri）
│   ├── common/                     # 公共类型、配置、工具
│   ├── domain/                     # 领域层（纯业务逻辑，零 IO）
│   ├── data-layer/                 # 数据层（PostgreSQL + Redis + Binance）
│   ├── data-puller/                # 后台行情/账户快照拉取
│   ├── repository/                 # 仓储层（数据库访问抽象）
│   ├── clients/                    # 外部客户端（Redis 缓存等）
│   ├── services/                   # 服务层（业务编排，typed errors）
│   ├── exchange-binance/            # Binance 交易所对接
│   ├── strategy-layer/             # 策略开发层
│   ├── trading-layer/              # 交易执行层
│   ├── risk-layer/                 # 风险管理层
│   ├── monitor-layer/              # 监控告警层
│   └── security/                   # 安全模块（加密、认证、审计）
├── src/                            # Vue3 前端
│   ├── views/                      # 页面（10 个视图）
│   ├── components/                 # 通用组件
│   ├── composables/                # 组合式函数
│   ├── stores/                     # Pinia 状态管理
│   └── services/                   # API 服务层
├── Cargo.toml                      # Workspace 配置
├── package.json                    # 前端依赖
├── rustfmt.toml                    # Rust 格式化配置
├── clippy.toml                     # Clippy lint 配置
└── .env.example                    # 环境变量模板
```

### 架构分层

```
┌─────────────────────────────────────────────────────────┐
│                    Vue3 前端 (src/)                      │
│         Element Plus + ECharts + Pinia + Axios          │
├─────────────────────────────────────────────────────────┤
│                 Tauri 命令层 (src-tauri/)                 │
│           src-tauri/src/commands/*.rs + ws_commands.rs  │
├─────────────────────────────────────────────────────────┤
│                  服务层 (crates/services)                 │
│     ServiceError typed errors + ServiceResult<T>        │
│  AccountService · AuthService · MarketService ·         │
│  BinanceService · RiskService · StrategyService ·              │
│  ConfigService                                          │
├─────────────────────────────────────────────────────────┤
│           领域层 (crates/domain)  零 IO 依赖              │
│        纯业务逻辑 · 类型定义 · 工具函数                    │
├──────────────┬──────────────┬───────────────────────────┤
│  数据层       │  交易层       │  风控层                    │
│  data-layer  │  trading-    │  risk-layer               │
│  repository  │  layer       │  · pre_trade              │
│  clients     │  exchange-   │  · real_time              │
│              │  binance     │  · post_trade · var       │
├──────────────┴──────────────┴───────────────────────────┤
│              基础设施 (PostgreSQL + Redis)                │
│   PostgreSQL RANGE 分区时序 · Redis 连接池缓存            │
└─────────────────────────────────────────────────────────┘
```

## 🚀 快速开始

### 环境要求

- **Rust**: 1.77+
- **Node.js**: 18+
- **PostgreSQL**: 14+
- **Redis**: 6+

### 安装步骤

1. **克隆项目**
```bash
git clone https://github.com/heheshang/quant_trading.git
cd quant_trading
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件，配置数据库连接、币安密钥等
```

3. **用 Docker 启动数据库组件（推荐，无本地 PG/Redis 时）**

   系统配置由 `dotenv` 从 `.env` 注入；`compose.yaml` 提供 PostgreSQL + Redis：
```bash
docker compose up -d postgres redis
# postgres → 127.0.0.1:15432，redis → 127.0.0.1:16379
```

   在 `.env` 中指向 Docker 端口：
```dotenv
DATABASE_HOST=127.0.0.1
DATABASE_PORT=15432
DATABASE_USERNAME=quant
DATABASE_PASSWORD=quant_password
DATABASE_NAME=quant_trading
REDIS_HOST=127.0.0.1
REDIS_PORT=16379
```

4. **安装前端依赖**
```bash
npm install
```

5. **运行迁移（幂等）**
```bash
cd src-tauri && DATABASE_HOST=127.0.0.1 DATABASE_PORT=15432 \
  DATABASE_USERNAME=quant DATABASE_PASSWORD=quant_password \
  DATABASE_NAME=quant_trading cargo run --bin migrate-db up
```

6. **运行开发环境**
```bash
# 方式1：同时启动前后端（自动后台迁移）
npm run tauri dev

# 方式2：分别启动
npm run dev          # 启动前端
cargo tauri dev      # 启动 Tauri 后端
```

7. **生产构建**
```bash
npm run build
npm run tauri build
```

## 📦 核心模块说明

### 1. 公共模块 (`crates/common`)

- **类型定义**：Order、Position、Account、StrategyParams、BacktestResult 等
- **配置管理**：AppConfig（数据库、Redis、交易、风控、Binance、安全配置）
- **工具函数**：时间处理、数学计算等

### 2. 领域层 (`crates/domain`)

- **纯业务逻辑**：零 IO 依赖，不引入 sqlx/reqwest/redis
- **类型与工具**：复用 common 层定义，保持领域纯净

### 3. 数据层 (`crates/data-layer`)

- **PostgreSQL**：订单、持仓、账户、策略、回测结果存储
- **Redis**：热点数据缓存，连接池管理（deadpool-redis）
- **Binance 数据源**：历史行情、实时行情接入
- **数据质量**：实时清洗、去重、异常检测
- **迁移管理**：SQL 迁移脚本自动执行

### 4. 仓储层 (`crates/repository`)

- **PostgresClient**：连接池封装，SQL 执行
- **MarketDataRepository**：行情数据仓储
- **RepositoryError**：仓储层 typed errors

### 5. 服务层 (`crates/services`)

- **业务编排**：组合 domain + repository + clients
- **Typed Errors**：`ServiceError`（16 个变体）+ `ServiceResult<T>`
- **7 个服务**：
  - `AccountService` — 账户、订单、持仓管理
  - `AuthService` — 登录、JWT、用户资料、密码管理
  - `MarketService` — 实时/历史行情
  - `BinanceService` — Binance 交易所封装
  - `RiskService` — 风控指标、配置、事前检查
  - `StrategyService` — 策略 CRUD、回测执行
  - `ConfigService` — 运行时配置读写

### 6. 策略层 (`crates/strategy-layer`)

- **策略接口**：统一的 `Strategy` trait + `StrategyContext`
- **技术指标**：SMA、EMA、RSI、MACD、布林带（命名常量，无魔法数字）
- **回测引擎**：高保真回测，包含滑点、手续费模拟
- **性能指标**：夏普比率、最大回撤、胜率、盈亏比

### 7. 交易层 (`crates/trading-layer`)

- **订单管理**：订单生命周期管理
- **执行引擎**：支持模拟盘和实盘切换
- **算法交易**：TWAP、VWAP、冰山订单（纸面已接线：`run_algorithmic_order` 拆分后经 `OrderProcessor` 逐单下单；实盘按普通市价/限价子单分批）
- **Binance 执行器**：对接 Binance 交易所下单

### 8. 风控层 (`crates/risk-layer`)

- **事前风控** (`pre_trade`)：资金检查、持仓限制、集中度控制
- **实时监控** (`real_time`)：账户风险、保证金、回撤监控
- **事后分析** (`post_trade`)：归因分析
- **VaR 计算** (`var`)：风险价值估算

### 9. 监控层 (`crates/monitor-layer`)

- **Prometheus 指标**：订单量、延迟、账户余额等
- **结构化日志**：基于 tracing 的分级日志系统
- **多渠道告警**：邮件、Webhook、企业微信

### 10. 安全模块 (`crates/security`)

- **加密**：AES-GCM 数据加密、Argon2 密码哈希
- **认证**：JWT 令牌生成与验证
- **API 密钥管理**：加密存储
- **审计日志**：操作追踪

### 11. Binance 交易所 (`crates/exchange-binance`)

- **REST API**：基于 `reqwest` + HMAC-SHA256 签名，覆盖行情、深度、K 线、下单、撤单、账户查询
- **WebSocket**：实时行情推送（Ticker、K 线、深度），自动重连
- **环境切换**：`BINANCE_ENVIRONMENT=spot`（现货）或 `futures`（合约），`BINANCE_ENABLE` 开关

## 💡 使用示例

### 创建策略

```rust
use strategy_layer::{Strategy, StrategyContext};
use quant_common::types::{Order, StrategyParams};

struct MyStrategy {
    params: StrategyParams,
}

#[async_trait]
impl Strategy for MyStrategy {
    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>> {
        // 策略逻辑
        Ok(vec![])
    }
}
```

### 运行回测

```rust
use strategy_layer::BacktestEngine;
use rust_decimal::Decimal;

let mut engine = BacktestEngine::new(
    Decimal::new(1000000, 0),  // 初始资金
    Decimal::new(3, 4),         // 手续费 0.03%
    Decimal::new(1, 4),         // 滑点 0.01%
);

let result = engine.run(&strategy, market_data).await?;
println!("年化收益: {}", result.annual_return);
println!("夏普比率: {}", result.sharpe_ratio);
println!("最大回撤: {}", result.max_drawdown);
```

### Typed Error 处理

```rust
use crates::services::error::{ServiceError, ServiceResult};

// 服务层返回 typed errors
async fn get_account(&self) -> ServiceResult<Account> {
    let client = self.postgres
        .as_ref()
        .ok_or(ServiceError::DatabaseNotConnected)?;
    // ...
}

// 调用方精确匹配错误类型
match service.get_account().await {
    Ok(account) => { /* ... */ },
    Err(ServiceError::DatabaseNotConnected) => { /* 重连 */ },
    Err(ServiceError::Database(e)) => { /* 记录 SQL 错误 */ },
    Err(e) => { /* 其他错误 */ },
}
```

## 🧪 测试

### 测试覆盖

- **前端**：33 个测试文件 / 423 个测试全部通过
- **Rust workspace**：编译、Clippy、测试通过；真实 PostgreSQL/Binance 集成测试默认 `#[ignore]`
- **风控层**：事前/事中/事后全流程测试

### 运行测试

```bash
# 全工作空间测试（排除需要 Binance 真实连接的 exchange-binance）
cargo test --workspace --exclude exchange-binance

# 单个 crate 测试
cargo test -p quant-services

# 带 clippy 检查
cargo clippy --workspace --exclude exchange-binance -- -D warnings
```

### 代码质量

```bash
# 格式化检查
cargo fmt --check

# Clippy lint（零警告）
cargo clippy --workspace --no-deps -- -D warnings
```

## 🔒 安全与合规

- ✅ 数据传输加密（TLS）
- ✅ API 密钥加密存储（AES-GCM）
- ✅ 密码哈希存储（Argon2）
- ✅ JWT 令牌认证
- ✅ 操作审计日志
- ✅ `.env.example` 无敏感信息泄露

## 📈 性能优化

- 向量化计算（Rust + ndarray）
- Redis 缓存热点数据（连接池）
- 异步 IO（Tokio）
- PostgreSQL 连接池管理
- 批量操作优化
- PostgreSQL RANGE 分区时序数据存储

## 🛠️ 开发工具配置

### Rust 配置

- `rustfmt.toml` — 代码格式化规则
- `clippy.toml` — Lint 严格度配置
- Workspace 统一依赖版本管理

### 依赖版本

| 依赖 | 版本 | 说明 |
|------|------|------|
| Rust | 1.77+ | Edition 2021 |
| Tauri | 2.0 | 桌面应用框架 |
| sqlx | 0.7 | PostgreSQL 异步驱动 |
| redis | 1.2 | Redis 客户端 |
| Vue | 3.4 | 前端框架 |
| Element Plus | - | UI 组件库 |
| ECharts | 5.4 | 图表库 |
| Pinia | 2.1 | 状态管理 |

## 🛣️ 开发路线图

### 第一阶段（已完成）✅
- [x] 项目架构搭建（13 crate 分层）
- [x] 数据层实现（PostgreSQL + Redis）
- [x] 策略层实现（回测引擎 + 技术指标）
- [x] 交易层实现（订单管理 + 算法交易）
- [x] 风控层实现（三层风控体系）
- [x] 监控层实现（Prometheus + 告警）
- [x] 安全模块（加密 + 认证 + 审计）
- [x] Binance 交易所集成
- [x] 前端界面（10 个视图）
- [x] Typed errors（服务层 ServiceError，命令层以 String 返回）
- [x] 测试覆盖（213 tests passing）

### 第二阶段（进行中）
- [ ] 数据库迁移脚本完善
- [ ] WebSocket 实时行情推送
- [ ] 更多技术指标
- [ ] 策略参数优化器
- [ ] 前端实时监控仪表盘

### 第三阶段（计划中）
- [ ] 机器学习模型集成
- [ ] 多市场支持
- [ ] 高频交易优化
- [ ] 分布式部署
- [ ] 策略市场

## 🤝 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

### 开发规范

- 遵循 `rustfmt.toml` 格式化规则
- Clippy 零警告
- 新功能必须附带测试
- 错误处理使用 typed errors（服务层禁止 `String` 错误传播；命令层以 `String` 返回给前端）
- 公共 API 必须有文档注释

## 📄 许可证

MIT License

## ⚠️ 免责声明

本软件仅供学习和研究使用。量化交易存在风险，请谨慎使用真实资金。过往业绩不代表未来收益。
