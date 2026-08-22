import { call } from './transport'

/**
 * WebSocket transport service (SoC).
 *
 * Manages lifecycle + subscription of the OKX market-data WebSocket
 * connection. Kept separate from `services/market.ts`, which only exposes
 * point-in-time market data reads.
 */

export function startMarketData(symbols: string[]): Promise<void> {
  return call<void>('start_market_data', { symbols })
}

export function subscribeMarketData(channel: string, symbol: string): Promise<void> {
  return call<void>('subscribe_market_data', { channel, symbol })
}

export function stopMarketData(): Promise<void> {
  return call<void>('stop_market_data')
}

export function subscribeChannel(symbol: string, channel: string): Promise<void> {
  return call<void>('subscribe_market_data', { channel, symbol })
}

export function unsubscribeChannel(symbol: string, channel: string): Promise<void> {
  return call<void>('unsubscribe_market_data', { channel, symbol })
}

export function getSubscriptions(): Promise<string[]> {
  return call<string[]>('get_subscriptions')
}
