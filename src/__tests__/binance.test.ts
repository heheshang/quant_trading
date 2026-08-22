import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockTauriInvoke,
  mockTauriInvokeError,
  resetTauriMocks,
  mockInvoke,
} from './mock-tauri'
import {
  getBinanceBalance,
  getBinanceCandles,
  checkBinanceStatus,
  startBinanceMarketData,
  stopBinanceMarketData,
  subscribeBinanceCandle,
  subscribeBinanceDepth,
  getBinanceSubscriptions,
} from '../services/binance'
import { placeBinanceOrder } from '../services/binanceOrder'
import type { BinanceOrder } from '../services/types'

describe('binance services', () => {
  beforeEach(() => resetTauriMocks())

  it('getBinanceBalance invokes get_binance_balance', async () => {
    mockTauriInvoke('get_binance_balance', [
      { asset: 'USDT', free: 100.5, locked: 0 },
    ])
    const res = await getBinanceBalance()
    expect(res).toEqual([{ asset: 'USDT', free: 100.5, locked: 0 }])
  })

  it('getBinanceCandles invokes get_binance_candles with args', async () => {
    mockTauriInvoke('get_binance_candles', [
      { open_time: 1, open: 1, high: 2, low: 0.5, close: 1.5, volume: 100, close_time: 2, quote_volume: 150, trades: 10 },
    ])
    const res = await getBinanceCandles('BTCUSDT', '1h', 100)
    expect(res).toHaveLength(1)
  })

  it('checkBinanceStatus invokes check_binance_status', async () => {
    mockTauriInvoke('check_binance_status', { connected: true })
    const res = await checkBinanceStatus()
    expect(res).toEqual({ connected: true })
  })

  it('placeBinanceOrder invokes place_binance_order with request', async () => {
    const order: BinanceOrder = {
      symbol: 'BTCUSDT',
      order_id: 123,
      client_order_id: 'ord-x',
      status: 'NEW',
      executed_qty: 0,
      cummulative_quote_qty: 0,
      price: 100,
    }
    mockTauriInvoke('place_binance_order', order)
    const res = await placeBinanceOrder({
      symbol: 'BTCUSDT',
      side: 'Buy',
      order_type: 'Limit',
      price: 100,
      quantity: 0.01,
    })
    expect(res.order_id).toBe(123)
  })

  it('surfaces command errors', async () => {
    mockTauriInvokeError('get_binance_balance', 'api down')
    await expect(getBinanceBalance()).rejects.toThrow('api down')
  })

  describe('binance websocket', () => {
    it('startBinanceMarketData invokes start_binance_market_data', async () => {
      mockTauriInvoke('start_binance_market_data', undefined)
      await expect(startBinanceMarketData()).resolves.toBeUndefined()
    })

    it('subscribeBinanceCandle passes symbol and interval', async () => {
      mockTauriInvoke('subscribe_binance_candle', null)
      await subscribeBinanceCandle('BTC-USDT', '1m')
      expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_candle', {
        symbol: 'BTC-USDT',
        interval: '1m',
      })
    })

    it('subscribeBinanceDepth passes symbol', async () => {
      mockTauriInvoke('subscribe_binance_depth', null)
      await subscribeBinanceDepth('BTC-USDT')
      expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_depth', {
        symbol: 'BTC-USDT',
      })
    })

    it('getBinanceSubscriptions returns list', async () => {
      mockTauriInvoke('get_binance_subscriptions', ['btcusdt@kline_1m'])
      const res = await getBinanceSubscriptions()
      expect(res).toEqual(['btcusdt@kline_1m'])
    })

    it('stopBinanceMarketData invokes stop_binance_market_data', async () => {
      mockTauriInvoke('stop_binance_market_data', undefined)
      await expect(stopBinanceMarketData()).resolves.toBeUndefined()
    })
  })
})
