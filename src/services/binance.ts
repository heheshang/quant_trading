import { call } from './transport'
import type { BinanceBalance, BinanceKline, BinanceOrderBook, BinanceStatus } from './types'

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

export function checkBinanceStatus(): Promise<BinanceStatus> {
  return call<BinanceStatus>('check_binance_status')
}

// ── WebSocket real-time (SoC: transport lifecycle in this module) ──

export function startBinanceMarketData(): Promise<void> {
  return call<void>('start_binance_market_data')
}

export function stopBinanceMarketData(): Promise<void> {
  return call<void>('stop_binance_market_data')
}

export function subscribeBinanceCandle(symbol: string, interval: string): Promise<void> {
  return call<void>('subscribe_binance_candle', { symbol, interval })
}

export function subscribeBinanceDepth(symbol: string): Promise<void> {
  return call<void>('subscribe_binance_depth', { symbol })
}

export function getBinanceSubscriptions(): Promise<string[]> {
  return call<string[]>('get_binance_subscriptions')
}
