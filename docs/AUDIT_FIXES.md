# 系统审计修复纪要

> 对「下单 / 回测 / 策略管理 / 风控 / 实盘」全链路审计后，按严重度修复的 27 项。
> 提交：`c30fcb8`（🔴）、`04e34d8`（🟠 ⑥-⑨）、`f83e73b`（🟠 ⑩-⑬）、`524becb`（🟡 ⑮-⑲）、`5646f55`（🟡 ⑱⑳21）。

## 2026-09-07 逻辑与架构复审

本轮修复：实盘查询失败虚构成交、订单终态和累计事实被覆盖、纸面撤单未持久化、落库失败遗留可执行订单、纸面与实盘持仓混用、市价风险估值绕过、风险配置故障放行、组合权益错位及盈亏比失真、策略保存绕过 CAS、用户元数据授权缺失、占位身份与跨会话缓存、行情启停竞争、实盘终态对账被镜像写入失败阻塞。

架构沿用现有分层：订单编排和数据库一致性放在服务层；命令层负责认证和适配；策略保存复用版本仓储；前端统一通过会话身份及请求代次隔离私有状态。无新增运行依赖、数据库迁移或真实交易操作。

### 验证结果

- `cargo test --workspace`：558 passed、0 failed、27 ignored；忽略项不计为通过。
- 显式运行隔离 PostgreSQL 回归：订单编排 8 项、风险服务 1 项、风险配置写失败 1 项，全部通过；使用独立容器和连接局部临时表。
- `npm test`：25 文件、269 项通过；`npm run build`（vue-tsc + Vite）通过。
- `cargo fmt --all -- --check` 通过；`cargo clippy --workspace --all-targets` 成功，仍有参数过多、冗余借用及 `as_bytes` 警告，未达到零警告门禁。登录节流的 MSRV 不兼容错误已修复，未在 Rust 1.77 工具链上重编整个依赖树。
- 浏览器运行真实 Vue 页面、使用离线 IPC 边界：无效身份不发布会话；策略页到设置页再返回保留策略；退出返回登录页、token 清除、策略不再显示。不是桌面 Tauri 或真实 Binance 联调。
- 执行/生命周期、风控/回测、授权/策略、实盘对账、会话、行情六个独立评审切片最终通过，无剩余 MUST FIX；评审为静态检查，运行验证由主代理执行。

### 明确保留的边界

- 本次未运行真实下单、真实交易所故障恢复、完整桌面运行或远程 CI，不部署、不提交 git。
- 策略 DTO 无客户端版本，只防服务端 CAS 竞争，不声称防跨编辑会话的陈旧覆盖。
- 实盘订单镜像缺失时，对账保留重试并能写入终态交易事实，但不新增自动重建镜像机制。
- Wiki 当前描述 Harness 流程和文件元数据；本次无其模型变更。业务及接口变化记录在本文件和 `docs/API_CONTRACT.md`，追踪见 `.harness/changes/fix-trading-audit-20260907/tasks.md`。

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
| 22 | **前端空数据（页面空数据）**：前端按 `1m` 读 K 线，但行情管线只落 `1h`（`DATA_PULLER_CANDLE_BARS=1H`），且 DB 中 `1m` 仅 BTC-USDT 有数据 → 切换任一其他标的（如 ETH-USDT）后 K 线/实时行情/预填价全空 | 前端所有 `1m` 读/订阅统一为 `1h`（`marketData.ts` prefetchCandles/pollDb/subscribeHeavy、`Binance.vue` prefillPrice/pollLiveData/startStream、`getKlines` 默认周期）；`domainSymbol` 幂等防 `BTC--USDT`；测试同步断言 `1h` | `src/stores/marketData.ts`/`src/views/Binance.vue`/`src/services/market.ts` |
| 23 | **实时行情表仍空（实时行情 还是没有数据）**：22 修复后 `pollLiveData` 仍只挂在 `startStream()` 内订阅之后——未点「开始」或 WS 启动/订阅任一失败（`subscribe_binance_candle` 在 WS 未启动时返回 NOT_INITIALIZED，启动失败又被 `.catch(()=>{})` 吞掉）→ try 块 reject → DB 轮询永不启动，即便 DB 已有 8760 行 `1h` 数据 | 行情显示与 WS 启停解耦：`onMounted` 无条件起 DB 轮询（立即 + 5s 定时）；`startStream`/`stopStream` 回归纯 WS 导入开关（订阅逐个 `.catch` 容错）；切换标的即刷新行情表；空态文案改「暂无 K 线数据」 | `src/views/Binance.vue` |
| 24 | **订单卡默认空（订单（0）数据为空）**：卡默认「活动」态只查未成交单（`get_active_orders`），而 DB 全部订单（4 笔 live + 21 笔 paper）均为 Filled/Expired 终态 → 空结果；2）时间列绑 `row.time`（仅 `BinanceOrder` 有该字段），DB `Order` 行恒显 `-` | 1）默认改「历史」态，挂载即 `get_recent_orders(100,'live')` 展示最近订单（含成交），与 `Trading.vue` 一致；2）时间列改用 `created_at`，`fmtTime` 兼容 ISO 字符串 | `src/views/Binance.vue` |
| 25 | **实盘风控订单符号被二次转换（`BTC--USDT`）**：前端已传 domain 符号，`live_request_to_order` 再经 `from_binance_symbol` 插 `-` → 风控按 symbol 精确匹配持仓落空：实盘卖单被误拒（Available: 0）、买单集中度少计（现金检查仍生效） | `live_request_to_order` 符号幂等：已含 `-` 原样使用，仅对 Binance 原始拼接（如 `BTCUSDT`）转换；新增 2 单测 | `src-tauri/src/commands/binance.rs` |
| 26 | **撤单按钮仅「活动」tab 渲染（`v-if="!ordersHistory"`）**：默认「历史」页未成交单无撤单入口 | 操作列改按行状态渲染：非终态（非 Filled/Cancelled/Rejected/Expired）任意 tab 均显示撤单 | `src/views/Binance.vue` |

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
