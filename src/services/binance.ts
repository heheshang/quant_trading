import { call } from './transport'
import type {
  AccountSnapshotRecord,
  BinanceBalance,
  BinanceKline,
  BinanceOrder,
  BinanceOrderBook,
  BinancePosition,
  BinanceStatus,
  LiveTrade,
} from './types'

/**
 * Binance exchange service (SoC).
 *
 * Account balance + market reads. Order placement/cancellation lives in
 * `services/binanceOrder.ts`.
 */

export function getBinanceBalance(): Promise<BinanceBalance[]> {
  return call<BinanceBalance[]>('get_binance_balance')
}

export function getBinanceCandles(
  symbol: string,
  interval: string,
  limit?: number,
): Promise<BinanceKline[]> {
  return call<BinanceKline[]>('get_binance_candles', { symbol, interval, limit })
}

export function getBinanceOrderBook(symbol: string, limit?: number): Promise<BinanceOrderBook> {
  return call<BinanceOrderBook>('get_binance_order_book', { symbol, limit })
}

export function getBinancePositions(symbol?: string): Promise<BinancePosition[]> {
  return call<BinancePosition[]>('get_binance_positions', { symbol: symbol ?? null })
}

export function getBinanceOrders(
  symbol: string,
  history?: boolean,
  limit?: number,
): Promise<BinanceOrder[]> {
  return call<BinanceOrder[]>('get_binance_orders', { symbol, history, limit })
}

export function getBinanceOrder(symbol: string, orderId: number): Promise<BinanceOrder> {
  return call<BinanceOrder>('get_binance_order', { symbol, orderId })
}

export function getBinanceInstruments(): Promise<unknown> {
  return call<unknown>('get_binance_instruments')
}

/** 本地记录的 live 单成交记录（策略关联 + 成交价/量）。 */
export function getLiveTrades(): Promise<LiveTrade[]> {
  return call<LiveTrade[]>('get_live_trades')
}

/** 全市场价格（`BTCUSDT → 77276.78`），用于持仓实时价格/市值补全。 */
export function getBinanceTickerPrices(): Promise<Record<string, number>> {
  return call<Record<string, number>>('get_binance_ticker_prices')
}

export function checkBinanceStatus(): Promise<BinanceStatus> {
  return call<BinanceStatus>('check_binance_status')
}

/** 账户权益快照历史（资产曲线）。 */
export function getAccountSnapshots(
  ccy: string,
  limit = 200,
): Promise<AccountSnapshotRecord[]> {
  return call<AccountSnapshotRecord[]>('get_account_snapshots', { ccy, limit })
}

/** 记录当前账户权益快照（资产曲线的点）。 */
export function recordAccountSnapshot(eq: number): Promise<void> {
  return call<void>('record_account_snapshot', { eq })
}

// ── WebSocket real-time (SoC: transport lifecycle in this module) ──

export function startBinanceMarketData(): Promise<void> {
  return call<void>('start_binance_market_data')
}

export function stopBinanceMarketData(): Promise<void> {
  return call<void>('stop_binance_market_data')
}

/** 启动用户数据流（`@userDataStream`），返回 listenKey。 */
export function startBinanceUserDataStream(): Promise<string> {
  return call<string>('start_binance_user_data_stream')
}

export function stopBinanceUserDataStream(): Promise<void> {
  return call<void>('stop_binance_user_data_stream')
}

export function subscribeBinanceCandle(symbol: string, interval: string): Promise<void> {
  return call<void>('subscribe_binance_candle', { symbol, interval })
}

export function subscribeBinanceDepth(symbol: string): Promise<void> {
  return call<void>('subscribe_binance_depth', { symbol })
}

export function subscribeBinanceTicker(symbol: string): Promise<void> {
  return call<void>('subscribe_binance_ticker', { symbol })
}

export function subscribeBinanceTrades(symbol: string): Promise<void> {
  return call<void>('subscribe_binance_trades', { symbol })
}

export function subscribeBinanceOrderbook(symbol: string): Promise<void> {
  return call<void>('subscribe_binance_orderbook', { symbol })
}

export function getBinanceSubscriptions(): Promise<string[]> {
  return call<string[]>('get_binance_subscriptions')
}
