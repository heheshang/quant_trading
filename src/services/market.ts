import { call } from './transport'
import type { MarketData } from './types'

/**
 * Market data service (SoC).
 *
 * Point-in-time market data reads. WebSocket lifecycle/subscription lives in
 * `services/ws.ts`.
 */

export function getMarketData(symbol: string): Promise<MarketData> {
  return call<MarketData>('get_market_data', { symbol })
}

export function getOkxRealtimeData(symbol: string): Promise<MarketData> {
  return call<MarketData>('get_okx_realtime_data', { symbol })
}

export function getOkxHistoricalData(
  symbol: string,
  start: string,
  end: string,
): Promise<MarketData[]> {
  return call<MarketData[]>('get_okx_historical_data', { symbol, start, end })
}
