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
  getBinancePositions,
  getBinanceOrders,
  getBinanceOrder,
  getBinanceInstruments,
  startBinanceMarketData,
  stopBinanceMarketData,
  subscribeBinanceCandle,
  subscribeBinanceDepth,
  subscribeBinanceTicker,
  subscribeBinanceTrades,
  subscribeBinanceOrderbook,
  getBinanceSubscriptions,
} from '../services/binance'
import { placeBinanceOrder, cancelBinanceOrder } from '../services/binanceOrder'
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

  it('getBinancePositions invokes get_binance_positions with symbol', async () => {
    mockTauriInvoke('get_binance_positions', [
      { symbol: 'BTCUSDT', position_amt: 0.001, entry_price: 50000, mark_price: 51000, un_realized_profit: 1, liquidation_price: 0, leverage: '10', margin_type: 'crossed', notional: 50, position_side: 'BOTH' },
    ])
    const res = await getBinancePositions('BTC-USDT')
    expect(res).toHaveLength(1)
    expect(res[0].symbol).toBe('BTCUSDT')
    expect(mockInvoke).toHaveBeenCalledWith('get_binance_positions', { symbol: 'BTC-USDT' })
  })

  it('getBinanceOrders invokes get_binance_orders with history flag', async () => {
    mockTauriInvoke('get_binance_orders', [
      { symbol: 'BTCUSDT', order_id: 1, client_order_id: 'x', status: 'NEW', executed_qty: 0, cummulative_quote_qty: 0, price: 50000, side: 'BUY', order_type: 'LIMIT', orig_qty: 0.01, time: 1700000000000 },
    ])
    const res = await getBinanceOrders('BTC-USDT', true, 50)
    expect(res).toHaveLength(1)
    expect(mockInvoke).toHaveBeenCalledWith('get_binance_orders', { symbol: 'BTC-USDT', history: true, limit: 50 })
  })

  it('getBinanceOrder invokes get_binance_order', async () => {
    mockTauriInvoke('get_binance_order', { symbol: 'BTCUSDT', order_id: 123, client_order_id: 'x', status: 'NEW', executed_qty: 0, cummulative_quote_qty: 0, price: 100 })
    const res = await getBinanceOrder('BTC-USDT', 123)
    expect(res.order_id).toBe(123)
    expect(mockInvoke).toHaveBeenCalledWith('get_binance_order', { symbol: 'BTC-USDT', orderId: 123 })
  })

  it('getBinanceInstruments invokes get_binance_instruments', async () => {
    mockTauriInvoke('get_binance_instruments', { symbols: [] })
    const res = await getBinanceInstruments()
    expect(res).toEqual({ symbols: [] })
  })

  it('cancelBinanceOrder invokes cancel_binance_order', async () => {
    mockTauriInvoke('cancel_binance_order', null)
    await cancelBinanceOrder('BTCUSDT', 123)
    expect(mockInvoke).toHaveBeenCalledWith('cancel_binance_order', { symbol: 'BTCUSDT', orderId: 123 })
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

    it('subscribeBinanceTicker passes symbol', async () => {
      mockTauriInvoke('subscribe_binance_ticker', null)
      await subscribeBinanceTicker('BTC-USDT')
      expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_ticker', {
        symbol: 'BTC-USDT',
      })
    })

    it('subscribeBinanceTrades passes symbol', async () => {
      mockTauriInvoke('subscribe_binance_trades', null)
      await subscribeBinanceTrades('BTC-USDT')
      expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_trades', {
        symbol: 'BTC-USDT',
      })
    })

    it('subscribeBinanceOrderbook passes symbol', async () => {
      mockTauriInvoke('subscribe_binance_orderbook', null)
      await subscribeBinanceOrderbook('BTC-USDT')
      expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_orderbook', {
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
