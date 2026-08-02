# 代码功能点审计

> 审计日期：2026-08-02
> 方法：逐个核对 Tauri 命令、前端 service、视图调用与 Rust 业务层，结合测试和静态检查验证。

## 1. 视图功能点

| 视图 | 主要功能 | 后端命令 | 状态 |
|------|----------|----------|------|
| Login | 登录、Token 验证、记住用户名 | `login` / `verify_token` | ✅ 已检查 |
| Dashboard | 账户概览、持仓、活跃订单、实时行情 | `get_account_info` / `get_positions` / `get_active_orders` | ✅ 已检查 |
| Strategy | 策略 CRUD、生命周期、批量操作、回测入口 | `get_strategies` / `save_strategy` / `deploy_strategy` / `start_strategy` / ... | ✅ 已检查并修复搜索/toggle |
| Backtest | 回测配置、执行、历史记录、导出 | `run_backtest` / `get_backtest_results` / `get_backtest_result` / `delete_backtest_result` | ✅ 已检查 |
| Trading | 模拟下单、OKX 余额/持仓/下单/K线/公告 | `submit_order` / `place_okx_order` / `cancel_okx_order` / `get_okx_*` | ✅ 已修复下单参数契约 |
| Risk | 风险指标、配置、事前检查、告警 | `get_risk_metrics` / `update_risk_config` / `pre_trade_check` / `get_alerts` | ✅ 已检查 |
| Monitor | 指标、告警、阈值、日志、WS 事件 | `get_metrics` / `get_alerts` / `acknowledge_alert` / `get_logs` | ✅ 已检查 |
| Settings | 全量配置读写、OKX 状态检测 | `get_config` / `update_config` / `check_okx_status` | ✅ 已检查 |
| Profile | 资料查询/更新、密码修改、2FA 入口 | `get_user_profile` / `update_profile` / `change_password` | ✅ 已修复会话清理 |
| Test | 系统自检 | `get_metrics` / `get_account_info` / `verify_token` | ✅ 已检查 |

## 2. 后端功能点

| 模块 | 功能点 | 状态 |
|------|--------|------|
| data-layer | PostgreSQL、行情仓储、数据质量、OKX 数据源 | ✅ 已检查；Redis 已收敛到 clients |
| clients | Redis 缓存单一实现 | ✅ 已收敛重复实现 |
| repository | 策略/回测仓储、CAS 状态更新 | ✅ 已检查 |
| services | Auth/Account/Market/Strategy/Risk/Okx/Config | ✅ 已检查；AppServices 装配去重 |
| strategy-layer | 指标、信号、回测、注册中心、调度器 | ✅ 已修复调度器 symbols 空实现 |
| trading-layer | OrderManager、执行引擎、算法单、OKX executor | ✅ 已接入 OKX 订单状态查询 |
| risk-layer | 事前/实时/事后/VaR | ✅ 已检查 |
| monitor-layer | 指标、日志、告警 | ✅ 已修复 HTTP 客户端与测试稳定性 |
| security | 加密、JWT、API key、审计 | ✅ 已检查 |
| exchange-okx | REST、WebSocket、模拟数据 | ✅ 已新增单频道退订 |

## 3. 本轮修复清单

- `StrategyScheduler` 从策略参数读取 `symbols`，不再使用空市场数据执行调度任务。
- `OkxExecutor::get_order_status` 已通过 `get_order_details` 查询并映射订单状态。
- `DataPuller` 重试在 `max_attempts = 0` 时不再 panic。
- `get_market_data` 不再把账户资金伪装成行情数据，数据源不可用时明确报错。
- OKX WebSocket 新增 `unsubscribe_public`，前端“取消订阅”不再停止整个连接。
- OKX WebSocket 支持运行中动态订阅、优雅停止、按配置选择实盘/模拟盘，并修复心跳饿死问题。
- 密码修改后通过 `useAuthStore().clearSession()` 清理统一会话，不再删除错误 key。
- `OkxPlaceOrderRequest` 修正为 Rust `camelCase` 契约，`sz`/`px` 使用字符串。
- `Trading.vue` 移除 `any` 并补齐 `AccountInfo`、`Order`、OKX 类型。
- `data-layer` 与 `clients` 的重复 Redis 客户端收敛到 `quant_clients::RedisCache`。
- `AppServices::new` 与 `with_config_path` 共用策略服务装配逻辑。
- WebSocket 退订命令已补充单元测试。
- Vite 产物已拆分为 `element-plus` / `echarts` / `vue-vendor` / `axios` chunk。
- `sqlx` 已从 0.7 升级到 0.8.6，并补齐 `tls-rustls-ring` TLS 后端；workspace 编译、测试、clippy 均通过。
- 新增 Docker 部署与一键测试；修正 `20240101000013` 在全新库上重复添加 `strategies` 主键的问题。

## 4. 仍待处理

- 2FA 页面仍是功能开发中占位。
