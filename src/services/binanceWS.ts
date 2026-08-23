import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { startBinanceMarketData, stopBinanceMarketData } from './binance'
import type { BinanceWsTicker, BinanceWsTrade, BinanceWsDepth, BinanceWsKline } from './types'

/** Payload shape of the `binance:status` event (backend emits `{ status }`). */
export interface BinanceStreamStatus {
  status: string
}

/** Handler router for the `binance:*` realtime events. */
export interface BinanceEventHandlers {
  onTicker?: (data: BinanceWsTicker) => void
  onTrade?: (data: BinanceWsTrade) => void
  onOrderBook?: (data: BinanceWsDepth) => void
  onCandle?: (data: BinanceWsKline) => void
  onStatus?: (status: BinanceStreamStatus) => void
  onError?: (error: string) => void
}

/**
 * Subscribe to every `binance:*` realtime event and route payloads to the
 * provided handlers. Returns one unlisten function per event; call them all to
 * detach the listeners.
 */
export async function listenToBinanceEvents(
  handlers: BinanceEventHandlers,
): Promise<UnlistenFn[]> {
  const unlisteners: UnlistenFn[] = []
  unlisteners.push(
    await listen<BinanceWsTicker>('binance:ticker', (ev) => handlers.onTicker?.(ev.payload)),
  )
  unlisteners.push(
    await listen<BinanceWsTrade>('binance:trade', (ev) => handlers.onTrade?.(ev.payload)),
  )
  unlisteners.push(
    await listen<BinanceWsDepth>('binance:orderbook', (ev) => handlers.onOrderBook?.(ev.payload)),
  )
  unlisteners.push(
    await listen<BinanceWsKline>('binance:kline', (ev) => handlers.onCandle?.(ev.payload)),
  )
  unlisteners.push(
    await listen<BinanceStreamStatus>('binance:status', (ev) => handlers.onStatus?.(ev.payload)),
  )
  unlisteners.push(await listen<string>('binance:error', (ev) => handlers.onError?.(ev.payload)))
  return unlisteners
}

/** Start the backend Binance WebSocket. */
export function startBinanceStream(): Promise<void> {
  return startBinanceMarketData()
}

/** Stop the backend Binance WebSocket. */
export function stopBinanceStream(): Promise<void> {
  return stopBinanceMarketData()
}
