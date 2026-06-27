import { invoke } from '@tauri-apps/api/core'
import type {
  AccountInfo,
  Alert,
  AppConfig,
  BacktestResult,
  BacktestResultSummaryRow,
  LogEntry,
  MarketData,
  OkxBalance,
  OkxCandle,
  OkxInstrument,
  OkxOrder,
  OkxPlaceOrderRequest,
  OkxPosition,
  Order,
  Position,
  RiskConfig,
  StrategyParams,
} from './types'

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config')
}

export function updateConfig(config: AppConfig): Promise<boolean> {
  return invoke<boolean>('update_config', { config })
}

export function getMarketData(symbol: string): Promise<MarketData> {
  return invoke<MarketData>('get_market_data', { symbol })
}

export function submitOrder(order: Order): Promise<string> {
  return invoke<string>('submit_order', { order })
}

export function getAccountInfo(): Promise<AccountInfo> {
  return invoke<AccountInfo>('get_account_info')
}

export function getPositions(): Promise<Position[]> {
  return invoke<Position[]>('get_positions')
}

export function getActiveOrders(): Promise<Order[]> {
  return invoke<Order[]>('get_active_orders')
}

export function runBacktest(
  strategy_id: string,
  start_date: string,
  end_date: string,
  initial_capital: number,
  commission_rate: number,
  slippage: number,
  symbols: string[],
): Promise<BacktestResult> {
  return invoke<BacktestResult>('run_backtest', {
    strategyId: strategy_id,
    startDate: start_date,
    endDate: end_date,
    initialCapital: initial_capital,
    commissionRate: commission_rate,
    slippage,
    symbols,
  })
}

export function getBacktestResults(
  limit: number,
  offset: number,
): Promise<BacktestResultSummaryRow[]> {
  return invoke<BacktestResultSummaryRow[]>('get_backtest_results', { limit, offset })
}

export function getBacktestResult(id: number): Promise<BacktestResult> {
  return invoke<BacktestResult>('get_backtest_result', { id: id.toString() })
}

export function deleteBacktestResult(id: number): Promise<boolean> {
  return invoke<boolean>('delete_backtest_result', { id: id.toString() })
}

export function getMetrics(): Promise<Record<string, number>> {
  return invoke<Record<string, number>>('get_metrics')
}

export function getAlerts(): Promise<Alert[]> {
  return invoke<Alert[]>('get_alerts')
}

export function acknowledgeAlert(alertId: number): Promise<boolean> {
  return invoke<boolean>('acknowledge_alert', { alertId: alertId.toString() })
}

export function getLogs(
  level?: string,
  limit?: number,
): Promise<LogEntry[]> {
  return invoke<LogEntry[]>('get_logs', { level, limit })
}

export function getStrategies(): Promise<StrategyParams[]> {
  return invoke<StrategyParams[]>('get_strategies')
}

export function saveStrategy(strategy: StrategyParams): Promise<string> {
  return invoke<string>('save_strategy', { strategy })
}

export function deleteStrategy(strategyId: string): Promise<boolean> {
  return invoke<boolean>('delete_strategy', { strategyId })
}

export function deployStrategy(strategyId: string): Promise<string> {
  return invoke<string>('deploy_strategy', { strategyId })
}

export function startStrategy(strategyId: string): Promise<string> {
  return invoke<string>('start_strategy', { strategyId })
}

export function stopStrategy(strategyId: string): Promise<string> {
  return invoke<string>('stop_strategy', { strategyId })
}

export function pauseStrategy(strategyId: string): Promise<string> {
  return invoke<string>('pause_strategy', { strategyId })
}

export function resumeStrategy(strategyId: string): Promise<string> {
  return invoke<string>('resume_strategy', { strategyId })
}

export function archiveStrategy(strategyId: string): Promise<string> {
  return invoke<string>('archive_strategy', { strategyId })
}

export function toggleStrategy(
  strategyId: string,
  enabled: boolean,
): Promise<boolean> {
  return invoke<boolean>('toggle_strategy', { strategyId, enabled })
}

export function getRiskMetrics(): Promise<Record<string, number>> {
  return invoke<Record<string, number>>('get_risk_metrics')
}

export function getRiskConfig(): Promise<RiskConfig> {
  return invoke<RiskConfig>('get_risk_config')
}

export function updateRiskConfig(config: RiskConfig): Promise<boolean> {
  return invoke<boolean>('update_risk_config', { config })
}

export function preTradeCheck(
  order: Order,
  account: AccountInfo,
  positions: Position[],
): Promise<boolean> {
  return invoke<boolean>('pre_trade_check', { order, account, positions })
}

export function login(username: string, password: string): Promise<string> {
  return invoke<string>('login', { username, password })
}

export function verifyToken(token: string): Promise<boolean> {
  return invoke<boolean>('verify_token', { token })
}

export function updateProfile(profileData: Record<string, unknown>): Promise<boolean> {
  return invoke<boolean>('update_profile', { profileData })
}

export function changePassword(
  currentPassword: string,
  newPassword: string,
  username?: string,
): Promise<boolean> {
  return invoke<boolean>('change_password', {
    currentPassword,
    newPassword,
    username,
  })
}

export function getUserProfile(username?: string): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_user_profile', { username })
}

export function getOkxBalance(ccy?: string): Promise<OkxBalance[]> {
  return invoke<OkxBalance[]>('get_okx_balance', { ccy })
}

export function getOkxPositions(instId?: string): Promise<OkxPosition[]> {
  return invoke<OkxPosition[]>('get_okx_positions', { instId })
}

export function placeOkxOrder(
  request: OkxPlaceOrderRequest,
): Promise<OkxOrder> {
  return invoke<OkxOrder>('place_okx_order', { request })
}

export function cancelOkxOrder(
  instId: string,
  ordId: string,
): Promise<boolean> {
  return invoke<boolean>('cancel_okx_order', { instId, ordId })
}

export function getOkxCandles(
  instId: string,
  bar?: string,
  limit?: number,
): Promise<OkxCandle[]> {
  return invoke<OkxCandle[]>('get_okx_candles', { instId, bar, limit })
}

export function getOkxInstruments(
  instType?: string,
): Promise<OkxInstrument[]> {
  return invoke<OkxInstrument[]>('get_okx_instruments', { instType })
}

export function checkOkxStatus(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('check_okx_status')
}

export function getOkxAnnouncements(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_okx_announcements')
}

export function executeOkxOrder(order: Order): Promise<string> {
  return invoke<string>('execute_okx_order', { order })
}

export function getOkxRealtimeData(symbol: string): Promise<MarketData> {
  return invoke<MarketData>('get_okx_realtime_data', { symbol })
}

export function getOkxHistoricalData(
  symbol: string,
  start: string,
  end: string,
): Promise<MarketData[]> {
  return invoke<MarketData[]>('get_okx_historical_data', { symbol, start, end })
}

export function startMarketData(symbols: string[]): Promise<void> {
  return invoke<void>('start_market_data', { symbols })
}

export function subscribeMarketData(channel: string, symbol: string): Promise<void> {
  return invoke<void>('subscribe_market_data', { channel, symbol })
}

export function stopMarketData(): Promise<void> {
  return invoke<void>('stop_market_data')
}

export function subscribeChannel(symbol: string, channel: string): Promise<void> {
  return invoke<void>('subscribe_market_data', { channel, symbol })
}

// NOTE: Backend has no independent "unsubscribe" command yet,
// so this calls stop_market_data (stops ALL subscriptions).
// TODO: Replace with a dedicated backend command when available.
export function unsubscribeChannel(_symbol: string, _channel: string): Promise<void> {
  return invoke<void>('stop_market_data')
}

export function getSubscriptions(): Promise<string[]> {
  return invoke<string[]>('get_subscriptions')
}
