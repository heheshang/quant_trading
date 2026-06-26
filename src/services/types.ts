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
}

export interface StrategyParams {
  strategy_id: string
  strategy_name: string
  strategy_type: StrategyType
  params: Record<string, unknown>
  enabled: boolean
  max_position: number
  max_daily_loss: number
  created_at: string
  updated_at: string
}

export type StrategyType =
  | 'TrendFollowing'
  | 'MeanReversion'
  | 'Arbitrage'
  | 'MarketMaking'
  | 'Statistical'
  | 'MachineLearning'
  | 'Custom'

export interface BacktestResult {
  id?: number
  strategy_id: string
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

export interface RiskMetrics {
  timestamp: string
  var_95: number
  var_99: number
  portfolio_volatility: number
  beta: number
  concentration_risk: number
  leverage: number
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
  okx: OkxConfig
}

export interface DatabaseConfig {
  host: string
  port: number
  username: string
  password: string
  database: string
  max_connections: number
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

export interface OkxConfig {
  api_key: string
  api_secret: string
  passphrase: string
  environment: string
  enable: boolean
}

export interface OkxBalance {
  ccy: string
  avail_bal: number
  frozen_bal: number
  eq: number
}

export interface OkxPosition {
  inst_id: string
  pos_side: string
  pos: number
  avg_px: number
  upl: number
  mark_px: number
}

export interface OkxPlaceOrderRequest {
  inst_id: string
  td_mode: string
  side: OrderSide
  ord_type: OrderType
  sz: number
  px?: number
}

export interface OkxOrder {
  ord_id: string
  inst_id: string
  side: string
  ord_type: string
  sz: number
  px: number
  state: string
  fill_sz: number
  fill_px: number
  fee: number
  c_time: string
}

export interface OkxCandle {
  ts: string
  o: number
  h: number
  l: number
  c: number
  vol: number
}

export interface OkxInstrument {
  inst_id: string
  inst_type: string
  uly: string
  base_ccy: string
  quote_ccy: string
  ct_val: number
  tick_sz: number
  lot_sz: number
  min_sz: number
}

export interface WsTicker {
  inst_id: string
  last: string
  last_sz: string
  ask_px: string
  ask_sz: string
  bid_px: string
  bid_sz: string
  open24h: string
  high24h: string
  low24h: string
  vol24h: string
  ts: string
}

export interface WsTrade {
  inst_id: string
  px: string
  sz: string
  side: string
  ts: string
}

export interface WsOrderBook {
  inst_id: string
  asks: [string, string][]
  bids: [string, string][]
  ts: string
}

export interface WsCandle {
  inst_id: string
  o: string
  h: string
  l: string
  c: string
  vol: string
  ts: string
}

export type WsConnectionStatus = 'connected' | 'reconnecting' | 'disconnected'

export interface ConnectionStatusEvent {
  status: WsConnectionStatus
  retry_in?: number
}
