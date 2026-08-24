import { call } from './transport'
import type { MarketData, MarketDataRecord, TickerSnapshotRecord } from './types'

/**
 * Market data service (SoC).
 *
 * Point-in-time market data reads (Binance/Postgres).
 */

export function getMarketData(symbol: string): Promise<MarketData> {
  return call<MarketData>('get_market_data', { symbol })
}

/** 标的代码下拉数据源：从数据库 market_data 读取不同的 instruments。 */
export function getSymbols(): Promise<string[]> {
  return call<string[]>('get_symbols')
}

/** 从数据库读取某标的/周期最新 K 线（remote WS 导入后前端从 DB 读）。 */
export function getKlines(
  symbol: string,
  timeframe = '1m',
  limit = 100,
): Promise<MarketDataRecord[]> {
  return call<MarketDataRecord[]>('get_klines', { symbol, timeframe, limit })
}

/** 从数据库读取某标的近 N 次 ticker 快照（分钟桶）。 */
export function getTickerSnapshots(
  instId: string,
  limit = 1,
): Promise<TickerSnapshotRecord[]> {
  return call<TickerSnapshotRecord[]>('get_ticker_snapshots', { inst_id: instId, limit })
}
