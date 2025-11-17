# OKX 交易所集成指南

本系统已完整集成 OKX 交易所 API，支持实盘和模拟盘交易。

## 功能特性

### 1. REST API 功能
- ✅ 账户余额查询
- ✅ 持仓信息查询
- ✅ 下单（市价单、限价单）
- ✅ 撤单
- ✅ 获取 K 线数据
- ✅ 获取交易对信息
- ✅ 完整的签名认证

### 2. WebSocket 实时数据
- ✅ Ticker 行情订阅
- ✅ 交易数据流订阅
- ✅ 订单簿订阅
- ✅ K 线实时更新
- ✅ 自动心跳保活
- ✅ 断线自动重连

### 3. 安全特性
- ✅ API 密钥加密存储
- ✅ HMAC-SHA256 签名
- ✅ 时间戳防重放攻击
- ✅ 审计日志记录

## 快速开始

### 1. 配置 API 密钥

编辑 `.env` 文件：

```bash
# OKX 模拟盘配置
OKX_API_KEY=your_api_key_here
OKX_API_SECRET=your_secret_here
OKX_PASSPHRASE=your_passphrase_here
OKX_ENVIRONMENT=demo
```

**获取 OKX API 密钥**：
1. 访问 [OKX 官网](https://www.okx.com)
2. 注册并完成身份验证
3. 进入 API 管理页面
4. 创建 API Key（建议先使用模拟盘）
5. 保存 API Key、Secret 和 Passphrase

### 2. 使用示例

#### REST API 示例

```rust
use exchange_okx::{OkxClient, types::*};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建客户端
    let client = OkxClient::new(
        "your_api_key".to_string(),
        "your_secret".to_string(),
        "your_passphrase".to_string(),
        OkxEnvironment::Demo,
    )?;

    // 查询余额
    let balances = client.get_account_balance(None).await?;
    for balance in balances {
        println!("Currency: {}, Available: {}", balance.ccy, balance.avail_eq);
    }

    // 下市价单
    let order_request = OkxPlaceOrderRequest {
        inst_id: "BTC-USDT".to_string(),
        td_mode: "cash".to_string(),
        side: "buy".to_string(),
        ord_type: "market".to_string(),
        sz: "0.001".to_string(),
        px: None,
        cl_ord_id: Some("my_order_001".to_string()),
    };

    let order = client.place_order(order_request).await?;
    println!("Order placed: {:?}", order);

    // 获取 K 线数据
    let candles = client.get_candles("BTC-USDT", "1m", Some(100)).await?;
    for candle in candles.iter().take(5) {
        println!("Time: {}, Close: {}", candle.ts, candle.close);
    }

    Ok(())
}
```

#### WebSocket 示例

```rust
use exchange_okx::{OkxWebSocket, types::OkxEnvironment};

#[tokio::main]
async fn main() -> Result<()> {
    // 创建 WebSocket 客户端
    let ws = OkxWebSocket::new(OkxEnvironment::Demo);

    // 订阅行情
    ws.subscribe_ticker("BTC-USDT").await?;
    ws.subscribe_trades("BTC-USDT").await?;
    ws.subscribe_candle("BTC-USDT", "1m").await?;

    // 启动连接
    ws.start().await?;

    // 接收消息
    loop {
        if let Some(msg) = ws.receive().await {
            match msg {
                WsMessage::Ticker(data) => {
                    println!("Ticker: {:?}", data);
                }
                WsMessage::Trades(data) => {
                    println!("Trades: {:?}", data);
                }
                WsMessage::Candle(data) => {
                    println!("Candle: {:?}", data);
                }
                WsMessage::Error(err) => {
                    eprintln!("Error: {}", err);
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
```

## API 限制

### 频率限制
- 公共接口：20次/2秒
- 私有接口（交易）：60次/2秒
- WebSocket：单个连接最多订阅 240 个频道

### 系统已实现的保护
- ✅ 请求速率限制
- ✅ 超时重试机制
- ✅ 连接池管理
- ✅ 错误处理和日志

## 支持的订单类型

1. **市价单** (`market`)：立即以最优价格成交
2. **限价单** (`limit`)：指定价格，等待成交
3. **只做 Maker** (`post_only`)：只接受 Maker 订单

## 支持的交易模式

1. **现货交易** (`cash`)
2. **全仓杠杆** (`cross`)
3. **逐仓杠杆** (`isolated`)

## 常见问题

### Q1: 如何切换到实盘？
修改环境变量：
```bash
OKX_ENVIRONMENT=live
```
**警告**：实盘交易有真实资金风险，请谨慎操作！

### Q2: 签名验证失败？
检查：
1. API Key、Secret、Passphrase 是否正确
2. 系统时间是否同步（误差不能超过30秒）
3. IP 是否在白名单中

### Q3: WebSocket 断线？
系统已实现自动重连机制，正常情况下会自动恢复。

### Q4: 如何测试？
建议使用模拟盘测试：
1. 注册 OKX 账号
2. 开通模拟盘功能
3. 获取模拟盘 API 密钥
4. 设置 `OKX_ENVIRONMENT=demo`

## 安全建议

1. ✅ **永远不要**将 API 密钥硬编码在代码中
2. ✅ **永远不要**将 `.env` 文件提交到 Git
3. ✅ 使用只读 API Key 进行查询操作
4. ✅ 限制 API Key 的 IP 白名单
5. ✅ 定期轮换 API 密钥
6. ✅ 设置合理的提现白名单
7. ✅ 小额资金测试后再加大资金

## 相关文档

- [OKX API 官方文档](https://www.okx.com/docs-v5/zh/)
- [OKX 费率说明](https://www.okx.com/fees)
- [安全最佳实践](../security/README.md)

## 技术支持

遇到问题？
1. 查看日志：`logs/quant-trading.log`
2. 检查 OKX API 状态页面
3. 提交 Issue 到项目仓库
