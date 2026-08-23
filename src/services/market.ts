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
