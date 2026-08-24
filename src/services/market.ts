import { call } from './transport'
import type { MarketData } from './types'

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
