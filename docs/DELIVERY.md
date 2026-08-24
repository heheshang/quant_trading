# 交付说明（审计 + 修复 + 增强）

> 本轮对本项目「下单 / 回测 / 策略管理 / 风控 / 实盘 / 前端」全链路进行了 4 轮平行审计 + 2 次深挖，
> 并完成全部修复与若干功能增强。以下为完整交付说明（含各提交对照）。

## 一、审计成果

| 轮次 | 范围 | 发现 |
|---|---|---|
| 系统审计 🔴🟠🟡 | 订单执行/回测/策略/风控/安全/数据层/实盘 | 23 项（🔴5 🟠8 🟡10） |
| 深挖 C | 回测/多标的 | 2 项（C1 critical 跨标的污染，C2 组合回测） |
| 深挖 UI | 前端 UI 层 | 21 项（数据/安全/表单/竞态/显示/清理） |

**审计结论**：未发现 SQL 注入（全参数化）与 DB N+1；crypto 内核（AES-256-GCM/Argon2/HS256+exp）健壮。

## 二、修复提交对照

### 🔴 严重（资金安全/数据丢失）— `c30fcb8`
| 项 | 修复 |
|---|---|
| 1 实盘下单未鉴权+绕过风控 | `place_binance_order`/`cancel_binance_order` 接鉴权 + `PreTradeRiskChecker` |
| 2 硬编码默认密钥 | `config.validate_secrets()` 拒绝占位/短密钥，启动 fail-fast |
| 3 纸面调度器跨 exchange | 永远纸面引擎 + 仅 `paper/algorithm`，跳过 `live` |
| 4 市价单硬编码价 100 | 无行情参考价 → fail-closed 拒绝 |
| 5 实盘对账 key 错位 | `update_order_status` 终态守卫 + 实盘镜像/终态回写 |

### 🟠 高危（策略/鉴权/执行/限流）— `04e34d8`(⑥⑦⑧⑨) + `f83e73b`(⑩⑪⑫⑬)
| 项 | 修复 |
|---|---|
| ⑥ 策略仓位全仓忽略持仓 | `net_quantity`（买入=目标−持仓、卖出=清仓），4 策略接入 |
| ⑦ 实盘策略管线 stub 账户 | 确认用 `make_live_pipeline`（真实风控） |
| ⑧ 敏感读接口无鉴权 | 账户/持仓/订单/审计/Binance 读接口补 `require_auth` |
| ⑨ `update_profile` 越权 | 目标绑定会话用户 + admin 限定 |
| ⑩ 加密 KDF 弱 | hex 解码存满熵 + Argon2id + legacy 兼容 |
| ⑪ TWAP/VWAP 切片 | 时间比例分布 + 末片吸收余量 |
| ⑫ gauge 双源 | 单一写者 |
| ⑬ 实盘 monitor | 变化才写 + 失败重试 + 限流退避 |

### 🟡 中低 — `524becb`(⑮⑯⑰⑲) + `5646f55`(⑱⑳21)
⑮ 分区到 2027 ｜ ⑯ 卖单风控字段一致 ｜ ⑰ token_version/登录节流 ｜ ⑱ 审计完整性 ｜ ⑲ 纸面限价成交价+超时 ｜ ⑳ 前端会话 fail-closed ｜ 21a WS TOCTOU

### 深挖 C1/C2（回测/多标的）— `d8efe7c` + `6f85ff7` + `ac2a745`
C1 跨标的污染（critical）→ signal history 只喂目标标的 + 调度器逐标的；C2 组合回测 → 按标的拆分 + 一次聚合 + 单测。

### 前端 UI（21 项）— `16e4660` + `bdd7352` + `a48def4` + `5ccffd2`
数据完整性（Pnl FIFO/市价单重放价/估值回退均价）、安全（配置导出抹敏感字段、CSV 防注入、JWT 落盘加密）、表单校验、竞态（refreshPrice/Monitor/Login）、图表 ResizeObserver、store 清理、USDT 计价、红涨绿跌按 locale。

### 功能增强（顺带交付）
0、订单 exchange 分类 & 按 tab 展示（`c638847`…`528e1ba`）——纸面/实盘/算法独立、类型徽标、下单带入类型。
1、监控页真实数据 + 双轴趋势（`ec1d2d7`…`94e6bf4`）。
2、Dashboard 今日收益改权益快照差值（`dae391a`）。
3、活跃订单 DB 读取 + 落库 fail-closed + 实盘镜像（`c7dfd41`…`6ce5294`）。
4、纸面卖单风控/限价成交价/超时（`7b11bd1`…`12a0c42`）。

## 三、验证状态

| 项 | 结果 |
|---|---|
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo test`（strategy/services/trading/risk/security/bin） | 全绿 |
| `cargo build -p quant-trading-system` | exit 0 |
| `vue-tsc` | 0 |
| `vitest` | **246/246 全绿** |

## 四、迁移提示
- 新增迁移 `20240101000024_add_order_exchange.sql`（orders.exchange）、`...25_add_market_data_partitions_2027.sql`。
- 新增依赖：`tauri-plugin-store`（前端安全存储）、`@tauri-apps/plugin-store`。
- 启动需设置强随机 `JWT_SECRET`/`ENCRYPTION_KEY`（≥32 字节），否则 fail-fast 拒绝启动：
  `export JWT_SECRET=$(openssl rand -hex 32); export ENCRYPTION_KEY=$(openssl rand -hex 32)`

## 五、待办/已知
- 21b 前端订阅进一步降频（限数量/降 orderbook 频率）。
- `orders.order_id BIGSERIAL` 显式插入未推进序列（仅当未来用自增时需同步）。
- OS keychain（stronghold/keychain）作为进一步升级（当前为后端密钥加密落盘）。
