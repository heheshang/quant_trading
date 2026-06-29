# 数据模型

> 核心文件结构、元数据模型、变更生命周期数据流。

## 文件级 ER 关系

```
.harness/
    │
    ├── agents/              ← 流程定义（编排逻辑）
    │   ├── owner-agent.md         ───→ 驱动所有 stage
    │   ├── evaluator-agent.md     ───→ 评审每个 stage 产物
    │   └── initializer-agent.md   ───→ 初始化 harness 环境
    │
    ├── scripts/             ← 可执行检查
    │   ├── init.sh               ───→ 会话启动，source detect-*
    │   ├── detect-build.sh       ───→ → 输出 BUILD_TOOL 变量
    │   ├── detect-platform.sh    ───→ → 输出 PLATFORM 变量
    │   ├── verify-qg.sh|py       ───→ → 输出 QG-1~8 结果
    │   └── gc-scan.sh            ───→ → 输出熵清理报告
    │
    ├── changes/{change}/    ← 变更实例（每个变更一套）
    │   ├── spec.md               ───→ 阶段 1 产物
    │   ├── design.md              ───→ 阶段 3 产物
    │   ├── summary.md             ───→ 阶段 6 产物（SSOT）
    │   ├── progress.md            ───→ 跨 Session 恢复
    │   ├── tasks.md               ───→ 任务分解
    │   ├── review.md              ───→ 阶段 2/4 产物
    │   ├── test-plan.md           ───→ 阶段 5 产物
    │   ├── contract.md            ───→ 冲刺约定
    │   ├── feature_list.json      ───→ 特性追踪
    │   └── deploy-log.md          ───→ 部署记录
    │
    ├── wiki/                ← 知识库（本文档）
    ├── rules/               ← 架构/编码规则
    ├── mcp/                 ← MCP 配置中心
    └── changes/template/    ← 模板副本（cp 起点）
```

## 核心数据模型

### 变更元数据（summary.md — 单一真相源）

| 字段 | 类型 | 说明 | 必填 |
|------|------|------|------|
| change_id | `{type}-{name}-{date}` | 变更唯一标识（目录名） | ✅ |
| type | `feat`/`fix`/`refactor`/`docs`/`test`/`chore` | 变更类型 | ✅ |
| status | `draft` / `in-review` / `in-progress` / `completed` / `cancelled` | 当前状态 | ✅ |
| stages | `stage-1` ~ `stage-6` | 各阶段完成状态 | ✅ |
| owner | `{agent/human}` | 执行角色 | ✅ |
| created_at | `{ISO date}` | 创建时间 | ✅ |
| completed_at | `{ISO date}` | 完成时间 | 状态=completed 时 |

### 任务模型（tasks.md — 阶段内分解）

| 字段 | 类型 | 说明 |
|------|------|------|
| task_id | `{N}` | 序号 |
| title | `[WHERE] [HOW] to [WHY]` | 标题（编码位置+行为+动机） |
| status | `pending` / `in_progress` / `completed` / `blocked` | 状态 |
| depends_on | `[N, M]` | 前置依赖 |

### 质量门禁结果（verify-qg.sh 输出）

| QG | 名称 | 检查方式 | 通过条件 |
|----|------|----------|----------|
| QG-1 | 编译检查 | `$BUILD_CHECK_CMD` | exit 0 |
| QG-2 | 类型检查 | TypeScript `tsc --noEmit` / Python `basedpyright` | error=0 |
| QG-3 | 代码风格 | `format_check_cmd` | exit 0 |
| QG-4 | 圈复杂度 | `radon cc` / `lizard` | A/B 级 |
| QG-5 | 长函数 | `radon raw` | 250 行以内 |
| QG-6 | 配置外部化 | grep 硬编码 | 0 命中 |
| QG-7 | 测试执行 | 测试框架 | all pass |
| QG-8 | 覆盖率 | `coverage report` | ≥80% |

### 平台检测输出（detect-platform.sh）

| 变量 | 类型 | 可能值 |
|------|------|--------|
| `PLATFORM` | string | `opencode` / `claude` / `codex` / `cursor` / `github-actions` / `generic` |
| `PLATFORM_CONFIDENCE` | int | 0-100 |
| `MCP_CONFIG_PATH` | string | 平台对应的 MCP 配置文件路径 |
| `CI_AVAILABLE` | bool | `true` / `false` |

### 构建检测输出（detect-build.sh）

| 变量 | 类型 | 可能值 |
|------|------|--------|
| `BUILD_TOOL` | string | `maven` / `gradle` / `pip` / `npm` / `cargo` / `go` / `unknown` |
| `BUILD_COMPILE_CMD` | string | 实际执行的编译命令 |
| `BUILD_CHECK_CMD` | string | 实际执行的检查命令 |

## 命名规范

### 目录命名
- 变更目录：`{type}-{name}-{date}`（如 `feat-price-filter-20240101`）
- Dry Run 目录：`dry-run-{date}`（如 `dry-run-20240101`）

### 文件命名
- Markdown：小写蛇形（`spec.md`, `test-plan.md`, `deploy-log.md`）
- 脚本：小写蛇形 + 语言扩展（`detect-build.sh`, `verify-qg.py`）
- 配置：标准平台名（`opencode.jsonc`, `claude.json`）

### 阶段命名
- 阶段文件：`review-v{N}.md`（评审版本迭代）
- 进程文件：`summary.md`（最终归档），`progress.md`（实时追踪）

## 数据字典

### 变更类型枚举

| 值 | 含义 | 说明 |
|-----|------|------|
| feat | 新功能 | 新增业务能力 |
| fix | 缺陷修复 | 修复现有功能问题 |
| refactor | 重构 | 不改变外部行为 |
| docs | 文档 | 文档新增/修改 |
| test | 测试 | 测试新增/修复 |
| chore | 基础建设 | CI/配置/工具链 |
| dry-run | 演练 | Harness 框架验证 |

### 阶段状态枚举

| 值 | 含义 | 后续动作 |
|-----|------|----------|
| pending | 未开始 | 进入该阶段 |
| in-progress | 进行中 | 继续当前操作 |
| blocked | 阻塞中 | 解析阻塞原因 |
| completed | 已完成 | 进入下一阶段或归档 |

## Rust 领域类型模型

> 应用代码的核心领域类型。Agent 在编码时必须使用这些类型，禁止用裸类型（`i64`、`String`）替代。

### Newtype ID 类型

| 类型 | 内部表示 | 用途 | 示例 |
|------|----------|------|------|
| `UserId(uuid::Uuid)` | Uuid v7 | 用户唯一标识 | `UserId::new()` |
| `PriceId(i64)` | i64 | 价格规则标识 | `PriceId(12345)` |
| `OrderId(uuid::Uuid)` | Uuid v7 | 订单标识 | `OrderId::new()` |
| `ItemId(i64)` | i64 | 商品标识 | `ItemId(67890)` |

### 值对象类型

| 类型 | 内部表示 | 约束 | 说明 |
|------|----------|------|------|
| `Money` | `i64`（分） | 非负 | 金额，禁止 `f64`。支持 `+`、`-`、`*`（标量乘法） |
| `Percent` | `u32`（万分比） | 0-10000 | 百分比，0 = 0%，10000 = 100% |
| `Quantity` | `u32` | 正整数 | 数量 |
| `Timestamp` | `chrono::NaiveDateTime` | UTC | 数据库时间戳 |
| `Email` | `String` | 验证格式 | 邮箱地址值对象 |

### 领域枚举

| 枚举 | 变体 | 说明 |
|------|------|------|
| `PriceRuleType` | `Fixed` / `Percentage` / `Tiered` | 价格规则类型 |
| `OrderStatus` | `Pending` / `Confirmed` / `Paid` / `Shipped` / `Delivered` / `Cancelled` | 订单状态（状态机） |
| `PaymentMethod` | `CreditCard` / `Alipay` / `WechatPay` / `BankTransfer` | 支付方式 |

### 仓储 trait 接口

```rust
// 定义在 domain crate，实现在 repository crate
pub trait PriceRepository: Send + Sync {
    async fn find_by_id(&self, id: &PriceId) -> Result<Option<Price>, RepoError>;
    async fn find_by_item_id(&self, item_id: &ItemId) -> Result<Vec<Price>, RepoError>;
    async fn save(&self, price: &Price) -> Result<PriceId, RepoError>;
    async fn delete(&self, id: &PriceId) -> Result<(), RepoError>;
}
```

### 错误类型层次

```
AppError (common crate, impl IntoResponse)
├── ServiceError (services crate)
│   ├── NotFound
│   ├── ValidationFailed
│   └── UpstreamError
├── RepoError (repository crate)
│   ├── RowNotFound
│   ├── DuplicateKey
│   └── DbError
└── ClientError (clients crate)
    ├── Timeout
    ├── ConnectionFailed
    └── StatusError
```

- 每层错误使用 `thiserror` 派生
- 通过 `From` trait 自动转换
- `AppError` 实现 `IntoResponse`，映射到 HTTP 状态码

## 数据库 Schema → Rust 字段映射

> 本节是 **SQL schema 与 Rust 代码列名一致性**的契约。任何列变更必须同步更新本表。
> 完整迁移命名规范见 [`migration-naming.md`](./migration-naming.md)。

### 迁移文件位置

- 生产迁移：`crates/data-layer/migrations/`
- 加载机制：`sqlx::migrate!("./migrations")`（`data-layer/src/postgres.rs::run_migrations`）
- 集成测试：`crates/data-layer/tests/migration_integration.rs`（`#[ignore]`，需真实 PG）
- 启动入口：`src-tauri/src/main.rs:119` 在应用启动时自动调用

### 表 → Rust 字段映射

| SQL 表 | SQL 列 | Rust struct | Rust 字段 | 查询文件 |
|--------|--------|-------------|-----------|---------|
| `users` | `user_id` | (无 struct，row.get) | row.get("user_id") | `services/auth_service.rs` |
| `users` | `role` | — | row.get("role") | `services/auth_service.rs:52` |
| `users` | `phone, full_name, company, address` | — | row.get(...) | `services/auth_service.rs:113-118` |
| `accounts` | `account_id, total_assets, available_cash, frozen_cash, market_value, total_pnl, daily_pnl, margin, margin_ratio, updated_at` | `Account` | 1:1 | `services/account_service.rs:26-58` |
| `orders` | `order_id, account_id, strategy_id, symbol, order_type, side, price, quantity, filled_quantity, commission, slippage, status, created_at, updated_at` | `Order` | 1:1 | `services/account_service.rs:69-129` |
| `positions` | `symbol, quantity, available_quantity, avg_price, market_value, unrealized_pnl, realized_pnl, updated_at` | `Position` | 1:1 | `services/account_service.rs:189-216` |
| `strategies` | `id, strategy_id, strategy_name, strategy_type, params, enabled, max_position, max_daily_loss, status, description, tags, symbols, instance_label, version, created_at, updated_at, user_id` | `StrategyRow` (sqlx::FromRow) | 1:1 | `repository/strategy_repository.rs:15-35` |
| `backtest_results` | `id, strategy_id, strategy_name, start_date, end_date, initial_capital, final_capital, total_return, annual_return, sharpe_ratio, max_drawdown, win_rate, profit_loss_ratio, total_trades, winning_trades, losing_trades, equity_curve, symbols, commission_rate, slippage, parameters_json, created_at` | `BacktestResultRow` | 1:1 | `repository/backtest.rs:14-36` |
| `market_data` | `id, instrument_id, timeframe, timestamp, open, high, low, close, volume, turnover, open_interest, bid_prices, bid_volumes, ask_prices, ask_volumes, created_at` | `MarketDataRecord` (data-layer) / `MarketData` (domain) | 1:1 | `data-layer/market_data_repo.rs:8-20`, `domain/types.rs:41-55` |
| `market_data` | `(instrument_id, timeframe, timestamp)` | — | UNIQUE 约束，支持 `ON CONFLICT DO NOTHING` | `data-layer/market_data_repo.rs:52` |
| `risk_config` | `id, var_confidence_level, max_position_size, max_daily_loss, max_drawdown, max_concentration, enable_pre_trade_check, enable_real_time_monitor, created_at, updated_at` | `RiskConfig` (domain) | 1:1 | `services/risk_service.rs:123-150` |
| `ticker_snapshots` | `id, instrument_id, ts, last_px, open_24h, high_24h, low_24h, vol_24h, vol_ccy_24h, change_24h, created_at` | `NewTickerSnapshot` | 1:1 | `data-layer/market_data_repo.rs:115-137` |
| `account_snapshots` | `id, ccy, ts, eq, cash_bal, avail_eq, frozen_bal, created_at` | `NewAccountSnapshot` | 1:1 | `data-layer/market_data_repo.rs:141-160` |
| `position_snapshots` | `id, inst_id, ts, pos, avg_px, upl, upl_ratio, mark_px, created_at` | `NewPositionSnapshot` | 1:1 | `data-layer/market_data_repo.rs:164-184` |
| `funding_rates` | `id, inst_id, ts, funding_rate, next_funding_rate, funding_time, created_at` | `NewFundingRate` | 1:1 | `data-layer/market_data_repo.rs:188-206` |
| `mark_prices` | `id, inst_id, ts, mark_px, idx_px, created_at` | `NewMarkPrice` | 1:1 | `data-layer/market_data_repo.rs:210-227` |
| `api_keys` | `id, user_id, exchange, api_key, encrypted_secret, passphrase, is_active, created_at, last_used` | (无直接查询) | — | — |
| `audit_logs` | `id, user_id, username, action, resource, details, ip_address, success, error_message, created_at` | `AuditLog` (security) | 1:1（DB 写入未启用） | `security/audit.rs` |
| `alerts` | `id, level, source, message, acknowledged, acknowledged_by, acknowledged_at, created_at` | (无直接查询) | — | — |

### 列名变更流程

1. **新增列**：先写 migration（`ALTER TABLE ... ADD COLUMN IF NOT EXISTS`）→ 再更新 Rust struct → 再更新本表
2. **重命名列**：必须分两步：先 `ADD COLUMN new` + 双写 → 后续 migration 删旧列 + 改 Rust
3. **删除列**：先删 Rust 引用 → 再写 migration `DROP COLUMN` → 标记本表为"已删除"
4. **CI 校验**：`.harness/scripts/verify-migrations.sh` 检查文件命名规范

### 已知 active 字段映射

| 字段 | SQL 类型 | Rust 类型 | 说明 |
|------|---------|-----------|------|
| `accounts.account_id` | BIGSERIAL | i64 | 不是 `id`！用 `account_id` |
| `orders.order_id` | BIGSERIAL | i64 | 不是 `id` |
| `strategies.strategy_id` | VARCHAR(100) | String | 字符串主键 |
| `strategies.id` | SERIAL | i32 | 新增自增 id（migration 013） |
| `strategies.tags/symbols` | JSONB | `Vec<String>` | 通过 serde_json 转换 |
| `users.user_id` | BIGSERIAL | i64 | 数字主键 |
| `users.role` | VARCHAR(50) | String | 单角色（不是 `roles TEXT[]`） |
| `market_data.instrument_id` | VARCHAR(50) | String | 不是 `symbol`！ |
| `market_data.timestamp` | TIMESTAMPTZ | `DateTime<Utc>` | 分区键 |
| `risk_config.enable_*` | BOOLEAN | bool | migration 016 新增 |
| `ticker_snapshots.ts` | TIMESTAMPTZ | `DateTime<Utc>` | 不是 `timestamp` |

