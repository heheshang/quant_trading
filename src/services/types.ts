export interface Instrument {
  symbol: string
  exchange: Exchange
  instrument_type: InstrumentType
  contract_multiplier: number
  tick_size: number
  lot_size: number
}

export type Exchange = 'SSE' | 'SZSE' | 'CFFEX' | 'SHFE' | 'DCE' | 'CZCE' | 'INE'

export type InstrumentType = 'Stock' | 'Future' | 'Option' | 'ETF' | 'Index' | 'Bond'

export interface MarketData {
  symbol: string
  timestamp: string
  open: number
  high: number
  low: number
  close: number
  volume: number
  turnover: number
  open_interest: number | null
  bid_prices: number[]
  bid_volumes: number[]
  ask_prices: number[]
  ask_volumes: number[]
}

export type OrderType = 'Market' | 'Limit' | 'StopLoss' | 'StopLimit' | 'TWAP' | 'VWAP' | 'Iceberg'

export type OrderSide = 'Buy' | 'Sell'

export type OrderStatus = 'Pending' | 'Submitted' | 'PartiallyFilled' | 'Filled' | 'Cancelled' | 'Rejected' | 'Expired'

export interface Order {
  order_id: number
  strategy_id: string
  strategy_name?: string
  symbol: string
  order_type: OrderType
  side: OrderSide
  price: number | null
  quantity: number
  filled_quantity: number
  status: OrderStatus
  created_at: string
  updated_at: string
  commission: number
  slippage: number
  /** 订单来源/种类：paper / live / algorithm / manual 等。 */
  exchange?: string
}

export interface Position {
  symbol: string
  quantity: number
  available_quantity: number
  avg_price: number
  market_value: number
  unrealized_pnl: number
  realized_pnl: number
  updated_at: string
}

export interface AccountInfo {
  account_id: number
  total_assets: number
  available_cash: number
  frozen_cash: number
  market_value: number
  total_pnl: number
  daily_pnl: number
  margin: number
  margin_ratio: number
  updated_at: string
  equity_history?: [string, number][]
}

export interface StrategyParams {
  strategy_id: string
  strategy_name: string
  strategy_type: StrategyType
  params: Record<string, unknown>
  enabled: boolean
  max_position: number
  max_daily_loss: number
  status: StrategyStatus
  description?: string
  tags: string[]
  symbols: string[]
  instance_label?: string
  created_at: string
  updated_at: string
}

/**
 * 策略状态枚举
 */
export type StrategyStatus = 
  | 'Draft'
  | 'Backtesting'
  | 'Deployed'
  | 'Running'
  | 'Paused'
  | 'Archived'

/**
 * 策略详细信息，扩展自 StrategyParams 并包含状态信息
 */
export interface StrategyDetail extends StrategyParams {
  status: StrategyStatus
  performance?: StrategyPerformance
  stats?: StrategyStatsResponse
}

/**
 * 策略性能指标
 */
export interface StrategyPerformance {
  total_pnl: number
  sharpe_ratio: number
  max_drawdown: number
  win_rate: number
  profit_loss_ratio: number
  total_trades: number
  winning_trades: number
  losing_trades: number
  average_win: number
  average_loss: number
  expectancy: number
  calmar_ratio: number
  sortino_ratio: number
  omega_ratio: number
  recovery_factor: number
  risk_adjusted_return: number
  information_ratio: number
  tracking_error: number
  volatility: number
  var_95: number
  var_99: number
  beta: number
  alpha: number
  updated_at: string
}

/**
 * 策略统计响应，包含聚合统计信息
 */
export interface StrategyStatsResponse {
  strategy_id: string
  strategy_name: string
  total_pnl: number
  total_return: number
  sharpe_ratio: number
  max_drawdown: number
  win_rate: number
  total_trades: number
  winning_trades: number
  losing_trades: number
  average_win: number
  average_loss: number
  expectancy: number
  calmar_ratio: number
  sortino_ratio: number
  omega_ratio: number
  recovery_factor: number
  risk_adjusted_return: number
  information_ratio: number
  tracking_error: number
  volatility: number
  var_95: number
  var_99: number
  beta: number
  alpha: number
  start_date: string
  end_date: string
  initial_capital: number
  final_capital: number
  annual_return: number
  total_trades_count: number
  winning_trades_count: number
  losing_trades_count: number
  average_win_amount: number
  average_loss_amount: number
  profit_loss_ratio: number
  created_at: string
  updated_at: string
}

/**
 * 批量操作结果
 */
export interface BatchOperationResult<T> {
  success: boolean
  message: string
  data?: T
  errors?: string[]
  affected_count: number
  timestamp: string
}

/**
 * 参数类型枚举
 */
export type ParamType = 'Number' | 'String' | { Select: string[] }

/**
 * 数值参数的范围约束
 */
export interface ParamRange {
  min: number
  max: number
  step?: number
}

/**
 * 策略参数的 Schema 定义，用于驱动动态表单渲染
 */
export interface ParameterSchema {
  name: string
  param_type: ParamType
  default: unknown
  range?: ParamRange
  description: string
}

/**
 * 策略类型信息，注册表元数据（匹配 Rust StrategyTypeInfo）
 */
export interface StrategyTypeInfo {
  type_name: string
  display_name: string
  description: string
  parameters: ParameterSchema[]
}

export type StrategyType =
  | 'TrendFollowing'
  | 'MeanReversion'
  | 'MACD'
  | 'RSI'

export interface BacktestResult {
  id?: number
  strategy_id: string
  strategy_name: string | null
  start_date: string
  end_date: string
  initial_capital: number
  final_capital: number
  total_return: number
  annual_return: number
  sharpe_ratio: number
  max_drawdown: number
  win_rate: number
  profit_loss_ratio: number
  total_trades: number
  winning_trades: number
  losing_trades: number
  equity_curve: [string, number][]
}

export interface BacktestResultSummaryRow {
  id: number
  strategy_id: string
  strategy_name: string | null
  start_date: string
  end_date: string
  total_return: number
  sharpe_ratio: number | null
  max_drawdown: number | null
  total_trades: number | null
  win_rate: number | null
  created_at: string
}
export interface BacktestResultsPage {
  rows: BacktestResultSummaryRow[]
  total: number
}

export interface RiskMetrics {
  timestamp: string
  var_95: number
  var_99: number
  var_confidence_level: number
  max_position_size: number
  max_daily_loss: number
  max_drawdown: number
  max_concentration: number
  [key: string]: number | string
}

export type AlertLevel = 'Info' | 'Warning' | 'Critical'

export interface Alert {
  alert_id: number
  level: AlertLevel
  source: string
  message: string
  timestamp: string
  acknowledged: boolean
}

export interface LogEntry {
  timestamp: string
  level: string
  message: string
  module: string | null
}

export interface AppConfig {
  app_name: string
  version: string
  debug: boolean
  database: DatabaseConfig
  redis: RedisConfig
  trading: TradingConfig
  risk: RiskConfig
  monitoring: MonitoringConfig
  security: SecurityConfig
  binance?: BinanceConfig
}

export interface DatabaseConfig {
  host: string
  port: number
  username: string
  password: string
  database: string
  max_connections: number
  connect_timeout_seconds?: number
}

export interface RedisConfig {
  host: string
  port: number
  password: string | null
  db: number
  pool_size: number
}

export interface TradingConfig {
  enable_paper_trading: boolean
  max_orders_per_second: number
  default_commission_rate: number
  default_slippage: number
  order_timeout_seconds: number
}

export interface RiskConfig {
  max_position_size: number
  max_daily_loss: number
  max_drawdown: number
  max_concentration: number
  enable_pre_trade_check: boolean
  enable_real_time_monitor: boolean
  var_confidence_level: number
}

export interface MonitoringConfig {
  enable_prometheus: boolean
  prometheus_port: number
  log_level: string
  alert_email: string | null
  alert_webhook: string | null
}

export interface SecurityConfig {
  enable_encryption: boolean
  jwt_secret: string
  token_expiry_hours: number
  enable_2fa: boolean
  allowed_ips: string[]
}

export interface BinanceConfig {
  api_key: string
  api_secret: string
  environment: string
  enable: boolean
  base_url: string | null
  ws_url: string | null
  key_type: string
  private_key_path: string | null
}

// ── Binance ──

export interface BinanceBalance {
  asset: string
  free: number
  locked: number
}

export interface BinanceKline {
  open_time: number
  open: number
  high: number
  low: number
  close: number
  volume: number
  close_time: number
  quote_volume: number
  trades: number
}

export interface BinanceOrderBook {
  symbol: string
  bids: [number, number][]
  asks: [number, number][]
}

export interface BinancePlaceOrderRequest {
  symbol: string
  side: 'Buy' | 'Sell'
  order_type: 'Market' | 'Limit'
  price?: number
  quantity: number
  strategy_id?: string
}
export interface BinanceOrder {
  symbol: string
  order_id: number
  client_order_id: string
  status: string
  executed_qty: number
  cummulative_quote_qty: number
  price: number
  side?: string
  order_type?: string
  orig_qty?: number
  time?: number
  update_time?: number
}

/** 账户权益快照（`account_snapshots`）。 */
export interface AccountSnapshotRecord {
  ccy: string
  ts: string
  eq: number | null
  cash_bal?: number | null
  avail_eq?: number | null
  frozen_bal?: number | null
  created_at: string
}

/** 数据库 K 线行（`market_data`，remote WS 导入后前端从 DB 读）。 */
export interface MarketDataRecord {
  id: number
  instrument_id: string
  timeframe: string
  timestamp: string
  open: number
  high: number
  low: number
  close: number
  volume: number
  created_at: string | null
}

/** 数据库 ticker 快照行（`ticker_snapshots`）。 */
export interface TickerSnapshotRecord {
  instrument_id: string
  ts: string
  last_px: number | null
  open_24h: number | null
  high_24h: number | null
  low_24h: number | null
  vol_24h: number | null
  vol_ccy_24h: number | null
  change_24h: number | null
  created_at: string | null
}

/** 数据库逐笔成交行（`stream_trades`）。 */
export interface StreamTradeRecord {
  id: number
  symbol: string
  price: number
  quantity: number
  trade_time: string
  is_buyer_maker: boolean
  created_at: string | null
}

/** 数据库订单簿快照行（`orderbook_snapshots`，bids/asks 为 JSON 数组字符串）。 */
export interface OrderbookSnapshotRecord {
  symbol: string
  bids: string
  asks: string
  ts: string
  created_at: string | null
}

/** 本地持久化的 live 单成交记录（策略关联 + 成交价/量）。 */
export interface LiveTrade {
  id: number
  order_id: number
  symbol: string
  strategy_id: string
  side: string
  price: number
  quantity: number
  filled_quantity: number
  status: string
  created_at: string
  updated_at: string
}

export interface BinancePosition {
  symbol: string
  position_amt: number
  entry_price: number
  mark_price: number
  un_realized_profit: number
  liquidation_price: number
  leverage: string
  margin_type: string
  notional: number
  position_side: string
}

export interface BinanceStatus {
  connected: boolean
}

export interface BinanceWsDepth {
  symbol: string
  bids: [number, number][]
  asks: [number, number][]
}

export interface BinanceWsKline {
  symbol: string
  interval: string
  open_time: number
  open: number
  high: number
  low: number
  close: number
  volume: number
  is_closed: boolean
}

export interface BinanceWsTicker {
  symbol: string
  last_price: number
  price_change: number
  price_change_percent: number
  high: number
  low: number
  open: number
  volume: number
  quote_volume: number
  event_time: number
}

export interface BinanceWsTrade {
  symbol: string
  price: number
  quantity: number
  trade_time: number
  is_buyer_maker: boolean
}

/** 用户数据流 `outboundAccountPosition`（余额变化）。 */
export interface BinanceWsAccountPosition {
  event_time: number
  balances: BinanceWsBalance[]
}
export interface BinanceWsBalance {
  asset: string
  free: number
  locked: number
}
/** 用户数据流 `executionReport`（订单变化）。 */
export interface BinanceWsOrderUpdate {
  symbol: string
  order_id: number
  client_order_id: string
  side: string
  order_type: string
  price: number
  quantity: number
  executed_quantity: number
  status: string
  event_time: number
}
