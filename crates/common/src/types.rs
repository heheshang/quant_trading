use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 交易标的信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub symbol: String,
    pub exchange: Exchange,
    pub instrument_type: InstrumentType,
    pub contract_multiplier: Decimal,
    pub tick_size: Decimal,
    pub lot_size: i32,
}

/// 交易所枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Exchange {
    SSE,  // 上交所
    SZSE, // 深交所
    CFFEX, // 中金所
    SHFE, // 上期所
    DCE,  // 大商所
    CZCE, // 郑商所
    INE,  // 上海国际能源交易中心
}

/// 标的类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstrumentType {
    Stock,      // 股票
    Future,     // 期货
    Option,     // 期权
    ETF,        // ETF
    Index,      // 指数
    Bond,       // 债券
}

/// 市场行情数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub turnover: Decimal,
    pub open_interest: Option<Decimal>,
    
    // Level 2 数据
    pub bid_prices: Vec<Decimal>,
    pub bid_volumes: Vec<Decimal>,
    pub ask_prices: Vec<Decimal>,
    pub ask_volumes: Vec<Decimal>,
}

/// 订单类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    Market,        // 市价单
    Limit,         // 限价单
    StopLoss,      // 止损单
    StopLimit,     // 止损限价单
    TWAP,          // 时间加权平均
    VWAP,          // 成交量加权平均
    Iceberg,       // 冰山单
}

/// 订单方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// 订单状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,          // 待提交
    Submitted,        // 已提交
    PartiallyFilled,  // 部分成交
    Filled,           // 完全成交
    Cancelled,        // 已撤单
    Rejected,         // 已拒绝
    Expired,          // 已过期
}

/// 订单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: Uuid,
    pub strategy_id: String,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub commission: Decimal,
    pub slippage: Decimal,
}

/// 持仓信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: Decimal,
    pub available_quantity: Decimal,
    pub avg_price: Decimal,
    pub market_value: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub updated_at: DateTime<Utc>,
}

/// 账户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub account_id: Uuid,
    pub total_assets: Decimal,
    pub available_cash: Decimal,
    pub frozen_cash: Decimal,
    pub market_value: Decimal,
    pub total_pnl: Decimal,
    pub daily_pnl: Decimal,
    pub margin: Decimal,
    pub margin_ratio: Decimal,
    pub updated_at: DateTime<Utc>,
}

/// 策略参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParams {
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: StrategyType,
    pub params: serde_json::Value,
    pub enabled: bool,
    pub max_position: Decimal,
    pub max_daily_loss: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StrategyType {
    TrendFollowing,     // 趋势跟踪
    MeanReversion,      // 均值回归
    Arbitrage,          // 套利
    MarketMaking,       // 做市
    Statistical,        // 统计套利
    MachineLearning,    // 机器学习
    Custom,             // 自定义
}

/// 回测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub strategy_id: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub initial_capital: Decimal,
    pub final_capital: Decimal,
    pub total_return: Decimal,
    pub annual_return: Decimal,
    pub sharpe_ratio: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub profit_loss_ratio: Decimal,
    pub total_trades: i32,
    pub winning_trades: i32,
    pub losing_trades: i32,
    pub equity_curve: Vec<(DateTime<Utc>, Decimal)>,
}

/// 风险指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub timestamp: DateTime<Utc>,
    pub var_95: Decimal,        // 95% VaR
    pub var_99: Decimal,        // 99% VaR
    pub portfolio_volatility: Decimal,
    pub beta: Decimal,
    pub concentration_risk: Decimal,
    pub leverage: Decimal,
}

/// 告警级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// 告警信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_id: Uuid,
    pub level: AlertLevel,
    pub source: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub module: Option<String>,
}
