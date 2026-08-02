import { invoke } from '@tauri-apps/api/core'
import type { MarketData } from './types'

/**
 * Market data service.
 *
 * Fetch single-symbol market data, OKX realtime/historical candles, and
 * manage WebSocket subscriptions (start/stop/subscribe/unsubscribe).
 */

export function getMarketData(symbol: string): Promise<MarketData> {
  return invoke<MarketData>('get_market_data', { symbol })
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

export function unsubscribeChannel(symbol: string, channel: string): Promise<void> {
  return invoke<void>('unsubscribe_market_data', { channel, symbol })
}

export function getSubscriptions(): Promise<string[]> {
  return invoke<string[]>('get_subscriptions')
}
