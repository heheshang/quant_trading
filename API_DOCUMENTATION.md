# 量化交易系统 API 文档

## 1. 概述

本文档描述了量化交易系统的 Tauri 命令接口，这些接口是前端与后端 Rust 服务通信的桥梁。

## 2. 认证相关接口

### 2.1 用户登录

**命令名称**: `login`

**功能描述**: 用户身份验证，成功后返回 JWT Token

**请求参数**:
```typescript
{
  username: string,  // 用户名
  password: string   // 密码
}
```

**返回值**:
```typescript
// 成功时返回 JWT Token
string

// 失败时返回错误信息
string
```

**使用示例**:
```typescript
import { invoke } from "@tauri-apps/api/core";

const token = await invoke("login", {
  username: "admin",
  password: "admin123"
});
```

### 2.2 验证 Token

**命令名称**: `verify_token`

**功能描述**: 验证 JWT Token 的有效性

**请求参数**:
```typescript
{
  token: string  // JWT Token
}
```

**返回值**:
```typescript
// Token 有效返回 true，否则返回 false
boolean
```

### 2.3 获取用户资料

**命令名称**: `get_user_profile`

**功能描述**: 获取当前登录用户的个人资料信息

**请求参数**: 无

**返回值**:
```typescript
{
  username: string,     // 用户名
  email: string,        // 邮箱
  phone: string,        // 手机号
  full_name: string,    // 姓名
  company: string,      // 公司
  address: string,      // 地址
  created_at: string,   // 创建时间
  last_login: string    // 最后登录时间
}
```

### 2.4 更新用户资料

**命令名称**: `update_profile`

**功能描述**: 更新用户个人资料信息

**请求参数**:
```typescript
{
  // 用户资料数据（JSON 格式）
  profile_data: any
}
```

**返回值**:
```typescript
// 更新成功返回 true，失败返回 false
boolean
```

### 2.5 修改密码

**命令名称**: `change_password`

**功能描述**: 修改用户登录密码

**请求参数**:
```typescript
{
  current_password: string,  // 当前密码
  new_password: string       // 新密码
}
```

**返回值**:
```typescript
// 修改成功返回 true，失败返回 false
boolean
```

## 3. 配置管理接口

### 3.1 获取系统配置

**命令名称**: `get_config`

**功能描述**: 获取当前系统配置信息

**请求参数**: 无

**返回值**:
```typescript
{
  database: {
    host: string,
    port: number,
    username: string,
    password: string,
    database: string,
    max_connections: number,
    connect_timeout_seconds: number
  },
  redis: {
    host: string,
    port: number,
    password: string,
    db: number
  },
  trading: {
    enable_paper_trading: boolean,
    max_orders_per_second: number,
    default_commission_rate: number,
    default_slippage: number
  },
  risk: {
    max_position_size: number,
    max_daily_loss: number,
    max_drawdown: number,
    enable_pre_trade_check: boolean,
    enable_real_time_monitor: boolean,
    var_confidence_level: number
  },
  security: {
    jwt_secret: string,
    token_expiry_hours: number,
    encryption_key: string
  }
}
```

### 3.2 更新系统配置

**命令名称**: `update_config`

**功能描述**: 更新系统配置信息

**请求参数**:
```typescript
{
  // 完整的配置对象
  config: AppConfig
}
```

**返回值**:
```typescript
// 更新成功返回 true，失败返回 false
boolean
```

## 4. 市场数据接口

### 4.1 获取市场数据

**命令名称**: `get_market_data`

**功能描述**: 获取指定标的的市场行情数据

**请求参数**:
```typescript
{
  symbol: string  // 标的代码
}
```

**返回值**:
```typescript
{
  symbol: string,           // 标的代码
  timestamp: string,        // 时间戳
  open: string,             // 开盘价
  high: string,             // 最高价
  low: string,              // 最低价
  close: string,            // 收盘价
  volume: string,           // 成交量
  turnover: string,         // 成交额
  open_interest: string,    // 持仓量（期货期权）
  bid_prices: string[],     // 买盘价格
  bid_volumes: string[],    // 买盘量
  ask_prices: string[],     // 卖盘价格
  ask_volumes: string[]     // 卖盘量
}
```

## 5. 交易管理接口

### 5.1 提交订单

**命令名称**: `submit_order`

**功能描述**: 提交交易订单

**请求参数**:
```typescript
{
  order_id: string,         // 订单ID
  strategy_id: string,      // 策略ID
  symbol: string,           // 标的代码
  order_type: string,       // 订单类型（Market, Limit等）
  side: string,             // 买卖方向（Buy, Sell）
  price: string,            // 价格（市价单可为空）
  quantity: string,         // 数量
  filled_quantity: string,  // 已成交数量
  status: string,           // 订单状态
  created_at: string,       // 创建时间
  updated_at: string,       // 更新时间
  commission: string,       // 手续费
  slippage: string          // 滑点
}
```

**返回值**:
```typescript
// 成功时返回订单ID
string

// 失败时返回错误信息
string
```

### 5.2 获取账户信息

**命令名称**: `get_account_info`

**功能描述**: 获取账户资金信息

**请求参数**: 无

**返回值**:
```typescript
{
  account_id: string,       // 账户ID
  total_assets: string,     // 总资产
  available_cash: string,   // 可用资金
  frozen_cash: string,      // 冻结资金
  market_value: string,     // 市值
  total_pnl: string,        // 总盈亏
  daily_pnl: string,        // 今日盈亏
  margin: string,           // 保证金
  margin_ratio: string,     // 保证金率
  updated_at: string        // 更新时间
}
```

### 5.3 获取持仓信息

**命令名称**: `get_positions`

**功能描述**: 获取当前持仓信息

**请求参数**: 无

**返回值**:
```typescript
[
  {
    symbol: string,             // 标的代码
    quantity: string,           // 持仓数量
    available_quantity: string, // 可用数量
    avg_price: string,          // 平均成本
    market_value: string,       // 市值
    unrealized_pnl: string,     // 浮动盈亏
    realized_pnl: string,       // 实现盈亏
    updated_at: string          // 更新时间
  }
]
```

### 5.4 获取活跃订单

**命令名称**: `get_active_orders`

**功能描述**: 获取当前未完成的订单列表

**请求参数**: 无

**返回值**:
```typescript
[
  {
    order_id: string,         // 订单ID
    strategy_id: string,      // 策略ID
    symbol: string,           // 标的代码
    order_type: string,       // 订单类型
    side: string,             // 买卖方向
    price: string,            // 价格
    quantity: string,         // 数量
    filled_quantity: string,  // 已成交数量
    status: string,           // 订单状态
    created_at: string,       // 创建时间
    updated_at: string,       // 更新时间
    commission: string,       // 手续费
    slippage: string          // 滑点
  }
]
```

## 6. 策略管理接口

### 6.1 获取所有策略

**命令名称**: `get_strategies`

**功能描述**: 获取系统中所有交易策略的信息

**请求参数**: 无

**返回值**:
```typescript
[
  {
    strategy_id: string,      // 策略ID
    strategy_name: string,    // 策略名称
    strategy_type: string,    // 策略类型
    params: any,              // 策略参数（JSON）
    enabled: boolean,         // 是否启用
    max_position: string,     // 最大仓位
    max_daily_loss: string,   // 最大日亏损
    created_at: string,       // 创建时间
    updated_at: string        // 更新时间
  }
]
```

### 6.2 保存策略

**命令名称**: `save_strategy`

**功能描述**: 创建或更新交易策略

**请求参数**:
```typescript
{
  strategy_id: string,      // 策略ID
  strategy_name: string,    // 策略名称
  strategy_type: string,    // 策略类型
  params: any,              // 策略参数（JSON）
  enabled: boolean,         // 是否启用
  max_position: string,     // 最大仓位
  max_daily_loss: string,   // 最大日亏损
  created_at: string,       // 创建时间
  updated_at: string        // 更新时间
}
```

**返回值**:
```typescript
// 成功时返回策略ID
string

// 失败时返回错误信息
string
```

### 6.3 删除策略

**命令名称**: `delete_strategy`

**功能描述**: 删除指定的交易策略

**请求参数**:
```typescript
{
  strategy_id: string  // 策略ID
}
```

**返回值**:
```typescript
// 删除成功返回 true，失败返回 false
boolean
```

### 6.4 启用/禁用策略

**命令名称**: `toggle_strategy`

**功能描述**: 启用或禁用指定的交易策略

**请求参数**:
```typescript
{
  strategy_id: string,  // 策略ID
  enabled: boolean      // 是否启用
}
```

**返回值**:
```typescript
// 操作成功返回 true，失败返回 false
boolean
```

### 6.5 运行回测

**命令名称**: `run_backtest`

**功能描述**: 对指定策略运行历史数据回测

**请求参数**:
```typescript
{
  strategy_id: string,  // 策略ID
  start_date: string,   // 开始日期
  end_date: string      // 结束日期
}
```

**返回值**:
```typescript
{
  strategy_id: string,        // 策略ID
  start_date: string,         // 开始日期
  end_date: string,           // 结束日期
  initial_capital: string,    // 初始资金
  final_capital: string,      // 最终资金
  total_return: string,       // 总收益率
  annual_return: string,      // 年化收益率
  sharpe_ratio: string,       // 夏普比率
  max_drawdown: string,       // 最大回撤
  win_rate: string,           // 胜率
  profit_loss_ratio: string,  // 盈亏比
  total_trades: number,       // 总交易次数
  winning_trades: number,     // 盈利交易次数
  losing_trades: number,      // 亏损交易次数
  equity_curve: [string, string][]  // 权益曲线（时间，价值）
}
```

## 7. 风险管理接口

### 7.1 获取风险指标

**命令名称**: `get_risk_metrics`

**功能描述**: 获取当前风险指标数据

**请求参数**: 无

**返回值**:
```typescript
{
  var_95: number,           // 95% VaR
  var_99: number,           // 99% VaR
  max_position_size: number, // 最大仓位限制
  max_daily_loss: number,   // 最大日亏损限制
  max_drawdown: number      // 最大回撤限制
}
```

### 7.2 获取风险配置

**命令名称**: `get_risk_config`

**功能描述**: 获取风险管理系统配置

**请求参数**: 无

**返回值**:
```typescript
{
  max_position_size: number,      // 最大仓位比例
  max_daily_loss: number,         // 最大日亏损比例
  max_drawdown: number,           // 最大回撤比例
  enable_pre_trade_check: boolean, // 是否启用事前检查
  enable_real_time_monitor: boolean, // 是否启用实时监控
  var_confidence_level: number    // VaR置信水平
}
```

### 7.3 更新风险配置

**命令名称**: `update_risk_config`

**功能描述**: 更新风险管理系统配置

**请求参数**:
```typescript
{
  max_position_size: number,      // 最大仓位比例
  max_daily_loss: number,         // 最大日亏损比例
  max_drawdown: number,           // 最大回撤比例
  enable_pre_trade_check: boolean, // 是否启用事前检查
  enable_real_time_monitor: boolean, // 是否启用实时监控
  var_confidence_level: number    // VaR置信水平
}
```

**返回值**:
```typescript
// 更新成功返回 true，失败返回 false
boolean
```

### 7.4 事前风控检查

**命令名称**: `pre_trade_check`

**功能描述**: 在提交订单前进行风险检查

**请求参数**:
```typescript
{
  order: Order,      // 订单信息
  account: Account,  // 账户信息
  positions: Position[]  // 持仓信息
}
```

**返回值**:
```typescript
// 检查通过返回 true，不通过返回 false
boolean
```

## 8. 监控告警接口

### 8.1 获取实时指标

**命令名称**: `get_metrics`

**功能描述**: 获取系统实时监控指标

**请求参数**: 无

**返回值**:
```typescript
{
  orders_total: number,      // 总订单数
  orders_filled: number,     // 已成交订单数
  orders_cancelled: number,  // 已取消订单数
  account_balance: number,   // 账户余额
  position_value: number,    // 持仓价值
  daily_pnl: number          // 今日盈亏
}
```

### 8.2 获取告警信息

**命令名称**: `get_alerts`

**功能描述**: 获取系统告警信息

**请求参数**: 无

**返回值**:
```typescript
[
  {
    alert_id: string,   // 告警ID
    level: string,      // 告警级别（Info, Warning, Critical）
    source: string,     // 告警来源
    message: string,    // 告警消息
    timestamp: string,  // 告警时间
    acknowledged: boolean  // 是否已确认
  }
]
```

### 8.3 确认告警

**命令名称**: `acknowledge_alert`

**功能描述**: 确认处理指定告警

**请求参数**:
```typescript
{
  alert_id: string  // 告警ID
}
```

**返回值**:
```typescript
// 确认成功返回 true，失败返回 false
boolean
```

### 8.4 获取日志信息

**命令名称**: `get_logs`

**功能描述**: 获取系统日志信息

**请求参数**:
```typescript
{
  level: string,  // 日志级别（可选）
  limit: number   // 返回条数限制（可选）
}
```

**返回值**:
```typescript
[
  {
    timestamp: string,  // 时间戳
    level: string,      // 日志级别
    message: string,    // 日志消息
    module: string      // 模块名称
  }
]
```

### 8.5 检查 Redis 状态

**命令名称**: `check_redis_status`

**功能描述**: 通过 Redis PING 检查缓存服务是否可用

**请求参数**: 无

**返回值**:
```typescript
// Redis 健康时返回 true，未初始化或 PING 失败时返回错误
boolean
```
