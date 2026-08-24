# 系统审计修复纪要

> 对「下单 / 回测 / 策略管理 / 风控 / 实盘」全链路审计后，按严重度修复的 23 项。
> 提交：`c30fcb8`（🔴）、`04e34d8`（🟠 ⑥-⑨）、`f83e73b`（🟠 ⑩-⑬）、`524becb`（🟡 ⑮-⑲）、`5646f55`（🟡 ⑱⑳21）。

## 🔴 严重（资金安全 / 数据丢失）

| # | 问题 | 修复 | 位置 |
|---|---|---|---|
| 1 | 实盘 Binance 下单未鉴权 + 完全绕过风控 | `place_binance_order`/`cancel_binance_order` 接 `require_auth`；下单前跑 `PreTradeRiskChecker`（现金/持仓/单日亏损/集中度） | `binance.rs` |
| 2 | 硬编码默认 JWT/加密密钥 → 可伪造/解密 | `config.validate_secrets()` 拒绝空/占位(`change_this`/`change_me`/`docker_test`)/短(<32B)密钥，启动 `panic` fail-fast | `config.rs`/`main.rs` |
| 3 | 纸面调度器处理所有 exchange 单 → 实盘模式重复下真实单 | 调度器永远用纸面引擎 + 只处理 `paper/algorithm`、跳过 `exchange='live'` | `order_processor.rs` |
| 4 | 市价单风控用硬编码价 100 → 高价值资产风控失效 | `resolve_market_data` 返回 `Result`；市价单无行情参考价 → fail-closed 拒绝 | `order_processor.rs` |
| 5 | 实盘单对账 key 错位 / 状态可被覆盖 | `update_order_status` 加终态守卫（不回退 Filled/Cancelled/Rejected/Expired）；实盘镜像+终态回写 | `account_service.rs`/`binance_ws.rs` |

## 🟠 高危

| # | 问题 | 修复 |
|---|---|---|
| 6 | 策略仓位 `max_position/last_close` 全仓、忽略持仓 | 新增 `net_quantity`（买入=目标−持仓、卖出=清仓）；4 策略接入，net≤0 跳过 |
| 7 | 实盘策略管线风控跑在清零 stub 账户 | 确认生产已用 `make_live_pipeline`(passthrough+OrderProcessor 真实风控)，无需改 |
| 8 | 大量敏感读接口无鉴权 | `get_account_info`/positions/orders/audit/binance-* 读接口接 `require_auth`/`require_role(admin)` |
| 9 | `update_profile` 越权 | 目标绑定会话用户，仅 admin 改他人；审计记录操作者 |
| 10 | 备用加密密钥弱 KDF | hex 长密钥解码存满熵（不再截断）；短密钥 Argon2id；保留 legacy_cipher 兼容解密历史数据 |
| 11 | TWAP/VWAP 切片区间截断/数量漂移 | 时间按比例分布 + 末片吸收余量（总量精确） |
| 12 | 指标 gauge 双源互相覆盖 | `position_value` 仅 `get_account_info` 写；`account_balance`/`daily_pnl` 由连续快照写者写 |
| 13 | 实盘 monitor 状态丢失 + 5s 无条件写/事件洪泛 + 限流 | 状态变化才写库/发事件；`get_order` 瞬时失败保留重试；`get_open_orders` 失败退避 10s |

## 🟡 中低

| # | 问题 | 修复 |
|---|---|---|
| 15 | `market_data` 分区只到 2026-12 | 新增迁移 25：2027 全年分区 |
| 16 | 卖单风控字段不一致（`available_quantity` vs `quantity`/`.abs()`） | position_limit/concentration 卖出统一用 `available_quantity` + clamp≥0 |
| 17 | 会话不校验 token_version / 登录无频率限制 | `verify_token` 失效清除会话（改密即下线）；登录节流（5 次失败→指数退避锁机 30min） |
| 18 | 审计完整性（`log_order_submit` 硬编码 success、被拒不审计、未知 action→Login） | `log_order_submit` 加 success/error；失败也审计；`AuditAction::Unknown` |
| 19 | 纸面限价成交价 `limit±滑点` 永远更差 / 单永不失效 | 成交价以市价为基础 + 限价 clamp；调度器消费 `order_timeout_seconds` → 超时 Expired |
| 20 | 前端 `restoreSession` fail-open 留失效 token | 改 fail-closed：无法确认有效即清除会话 |
| 21 | WS 启停 TOCTOU / 订阅洪泛 | `compare_exchange` 原子抢锁 + 失败释放锁；概览只订阅 ticker，重流仅活跃标的 |

## 深挖补充（回测/多标的）

| # | 问题 | 修复 |
|---|---|---|
| C1 | **跨标的污染（critical）**：回测/调度器把多标的 bar 交错拼进单一 buffer，策略对整个混合序列算指标却只交易 `market_data[0].symbol` → 指标完全错误 | `BacktestEngine` signal history 只喂目标标的；调度器**逐标的**生成信号（每标的各自历史） |
| C2 | 多标的组合回测 | `run_backtest_multi` **按标的拆分**：每个 `(策略,标的)` 独立回测（各自历史避免污染），初始资本均分，`aggregate_portfolio` 一次聚合（权益按时间戳求和、成交合并、以全量初资本重算指标）；单标的零变化 |

## 审计结论
- **未发现** SQL 注入（全 `sqlx::query().bind()` 参数化）与 DB N+1。
- crypto 内核（AES-256-GCM 随机 nonce、Argon2、HS256+exp）本身健壮。
- 主要系统性风险集中在：实盘下单链路鉴权/风控、公开默认密钥、纸面引擎跨 exchange、会话/登录安全、审计盲区。

## 待办/调优项
- **21b** 前端 per-symbol 订阅洪泛的进一步降频（限数量/降低 orderbook 更新频率）。
- `orders.order_id BIGSERIAL` 显式插入未推进序列（仅当未来改用自增时需同步序列）。
