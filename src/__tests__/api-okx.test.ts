import { describe, it, expect, beforeEach } from 'vitest'
import {
  mockTauriInvoke,
  mockTauriInvokeError,
  resetTauriMocks,
  mockInvoke,
} from './mock-tauri'
import {
  mockOkxBalanceList,
  mockOkxCandleList,
  mockOkxPositionList,
  mockOkxOrder,
  mockOkxInstrumentList,
  mockLargeNumberBalance,
  mockEmptyPositionList,
} from './mock-okx-data'
import type {
  OkxBalance,
  OkxPlaceOrderRequest,
  Order,
  MarketData,
} from '../services/types'
import {
  getOkxBalance,
  getOkxPositions,
  placeOkxOrder,
  cancelOkxOrder,
  getOkxCandles,
  getOkxInstruments,
  checkOkxStatus,
  getOkxAnnouncements,
  executeOkxOrder,
  getOkxRealtimeData,
  getOkxHistoricalData,
  subscribeMarketData,
  stopMarketData,
} from '../services/api'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mockMarketData(overrides: Partial<MarketData> = {}): MarketData {
  return {
    symbol: 'BTC-USDT',
    timestamp: '2025-06-27T12:00:00Z',
    open: 50000,
    high: 51000,
    low: 49000,
    close: 50500,
    volume: 1000,
    turnover: 50000000,
    open_interest: null,
    bid_prices: [49900, 49800],
    bid_volumes: [1, 2],
    ask_prices: [50100, 50200],
    ask_volumes: [1.5, 2.5],
    ...overrides,
  }
}

function mockOrder(overrides: Partial<Order> = {}): Order {
  return {
    order_id: 1,
    strategy_id: 'strat-1',
    symbol: 'BTC-USDT',
    order_type: 'Limit',
    side: 'Buy',
    price: 50000,
    quantity: 0.1,
    filled_quantity: 0,
    status: 'Pending',
    created_at: '2025-06-27T12:00:00Z',
    updated_at: '2025-06-27T12:00:00Z',
    commission: 0,
    slippage: 0,
    ...overrides,
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('OKX API wrappers', () => {
  beforeEach(() => {
    resetTauriMocks()
  })

  // -----------------------------------------------------------------------
  // 1. getOkxBalance
  // -----------------------------------------------------------------------
  describe('getOkxBalance', () => {
    it('returns OkxBalance[] on success', async () => {
      const mockData = mockOkxBalanceList(3)
      mockTauriInvoke('get_okx_balance', mockData)

      const result = await getOkxBalance()

      expect(result).toHaveLength(3)
      expect(result[0]).toHaveProperty('ccy', 'BTC')
      expect(result[0]).toHaveProperty('cashBal')
      expect(result[0]).toHaveProperty('frozenBal')
      expect(result[0]).toHaveProperty('eq')
    })

    it('passes ccy argument to Tauri command', async () => {
      mockTauriInvoke('get_okx_balance', mockOkxBalanceList(1))

      await getOkxBalance('BTC')

      expect(mockInvoke).toHaveBeenCalledWith('get_okx_balance', { ccy: 'BTC' })
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('get_okx_balance', 'API error')

      await expect(getOkxBalance()).rejects.toThrow('API error')
    })
  })

  // -----------------------------------------------------------------------
  // 2. getOkxPositions
  // -----------------------------------------------------------------------
  describe('getOkxPositions', () => {
    it('returns OkxPosition[] on success', async () => {
      const mockData = mockOkxPositionList(3)
      mockTauriInvoke('get_okx_positions', mockData)

      const result = await getOkxPositions()

      expect(result).toHaveLength(3)
      expect(result[0]).toHaveProperty('inst_id', 'BTC-USDT')
      expect(result[0]).toHaveProperty('pos_side')
      expect(result[0]).toHaveProperty('pos')
    })

    it('passes instId argument to Tauri command', async () => {
      mockTauriInvoke('get_okx_positions', mockOkxPositionList(1))

      await getOkxPositions('BTC-USDT')

      expect(mockInvoke).toHaveBeenCalledWith('get_okx_positions', {
        instId: 'BTC-USDT',
      })
    })

    it('returns empty array for empty positions', async () => {
      mockTauriInvoke('get_okx_positions', mockEmptyPositionList())

      const result = await getOkxPositions()

      expect(result).toEqual([])
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('get_okx_positions', 'Network error')

      await expect(getOkxPositions()).rejects.toThrow('Network error')
    })
  })

  // -----------------------------------------------------------------------
  // 3. placeOkxOrder
  // -----------------------------------------------------------------------
  describe('placeOkxOrder', () => {
    const sampleRequest: OkxPlaceOrderRequest = {
      inst_id: 'BTC-USDT',
      td_mode: 'cross',
      side: 'Buy',
      ord_type: 'Limit',
      sz: 0.1,
      px: 50000,
    }

    it('returns OkxOrder on success', async () => {
      const mockData = mockOkxOrder()
      mockTauriInvoke('place_okx_order', mockData)

      const result = await placeOkxOrder(sampleRequest)

      expect(result).toHaveProperty('ord_id', 'ord-12345')
      expect(result).toHaveProperty('inst_id', 'BTC-USDT')
      expect(result).toHaveProperty('side', 'buy')
    })

    it('passes request to Tauri command', async () => {
      mockTauriInvoke('place_okx_order', mockOkxOrder())

      await placeOkxOrder(sampleRequest)

      expect(mockInvoke).toHaveBeenCalledWith('place_okx_order', {
        request: sampleRequest,
      })
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('place_okx_order', 'Order rejected')

      await expect(placeOkxOrder(sampleRequest)).rejects.toThrow(
        'Order rejected',
      )
    })
  })

  // -----------------------------------------------------------------------
  // 4. cancelOkxOrder
  // -----------------------------------------------------------------------
  describe('cancelOkxOrder', () => {
    it('returns boolean true on success', async () => {
      mockTauriInvoke('cancel_okx_order', true)

      const result = await cancelOkxOrder('BTC-USDT', 'ord-12345')

      expect(result).toBe(true)
    })

    it('returns boolean false when cancellation fails', async () => {
      mockTauriInvoke('cancel_okx_order', false)

      const result = await cancelOkxOrder('BTC-USDT', 'ord-99999')

      expect(result).toBe(false)
    })

    it('passes instId and ordId to Tauri command', async () => {
      mockTauriInvoke('cancel_okx_order', true)

      await cancelOkxOrder('BTC-USDT', 'ord-12345')

      expect(mockInvoke).toHaveBeenCalledWith('cancel_okx_order', {
        instId: 'BTC-USDT',
        ordId: 'ord-12345',
      })
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('cancel_okx_order', 'Cancel failed')

      await expect(
        cancelOkxOrder('BTC-USDT', 'ord-12345'),
      ).rejects.toThrow('Cancel failed')
    })
  })

  // -----------------------------------------------------------------------
  // 5. getOkxCandles
  // -----------------------------------------------------------------------
  describe('getOkxCandles', () => {
    it('returns OkxCandle[] with bar and limit params', async () => {
      const mockData = mockOkxCandleList(10)
      mockTauriInvoke('get_okx_candles', mockData)

      const result = await getOkxCandles('BTC-USDT', '1H', 10)

      expect(result).toHaveLength(10)
      expect(result[0]).toHaveProperty('ts')
      expect(result[0]).toHaveProperty('o')
      expect(result[0]).toHaveProperty('h')
      expect(result[0]).toHaveProperty('l')
      expect(result[0]).toHaveProperty('c')
      expect(result[0]).toHaveProperty('vol')
    })

    it('calls with correct Tauri command and args', async () => {
      mockTauriInvoke('get_okx_candles', mockOkxCandleList(5))

      await getOkxCandles('BTC-USDT', '1H', 5)

      expect(mockInvoke).toHaveBeenCalledWith('get_okx_candles', {
        instId: 'BTC-USDT',
        bar: '1H',
        limit: 5,
      })
    })

    it('calls with default params (undefined bar and limit)', async () => {
      mockTauriInvoke('get_okx_candles', mockOkxCandleList())

      await getOkxCandles('ETH-USDT')

      expect(mockInvoke).toHaveBeenCalledWith('get_okx_candles', {
        instId: 'ETH-USDT',
        bar: undefined,
        limit: undefined,
      })
    })
  })

  // -----------------------------------------------------------------------
  // 6. getOkxInstruments
  // -----------------------------------------------------------------------
  describe('getOkxInstruments', () => {
    it('returns OkxInstrument[] with default params', async () => {
      const mockData = mockOkxInstrumentList(5)
      mockTauriInvoke('get_okx_instruments', mockData)

      const result = await getOkxInstruments()

      expect(result).toHaveLength(5)
      expect(result[0]).toHaveProperty('inst_id', 'BTC-USDT')
      expect(result[0]).toHaveProperty('inst_type', 'SPOT')
    })

    it('passes instType to Tauri command', async () => {
      mockTauriInvoke('get_okx_instruments', mockOkxInstrumentList(3))

      await getOkxInstruments('FUTURES')

      expect(mockInvoke).toHaveBeenCalledWith('get_okx_instruments', {
        instType: 'FUTURES',
      })
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('get_okx_instruments', 'Instruments error')

      await expect(getOkxInstruments()).rejects.toThrow('Instruments error')
    })
  })

  // -----------------------------------------------------------------------
  // 7. checkOkxStatus
  // -----------------------------------------------------------------------
  describe('checkOkxStatus', () => {
    it('returns status object on success', async () => {
      const mockStatus = {
        connected: true,
        demo_trading: true,
        timestamp: '2025-06-27T12:00:00Z',
      }
      mockTauriInvoke('check_okx_status', mockStatus)

      const result = await checkOkxStatus()

      expect(result).toHaveProperty('connected', true)
      expect(result).toHaveProperty('demo_trading', true)
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('check_okx_status', 'Connection refused')

      await expect(checkOkxStatus()).rejects.toThrow('Connection refused')
    })
  })

  // -----------------------------------------------------------------------
  // 8. getOkxAnnouncements
  // -----------------------------------------------------------------------
  describe('getOkxAnnouncements', () => {
    it('returns announcement data on success', async () => {
      const mockAnnouncements = [
        { title: 'Maintenance', content: 'Scheduled upgrade', date: '2025-06-28' },
      ]
      mockTauriInvoke('get_okx_announcements', mockAnnouncements)

      const result = await getOkxAnnouncements()

      // getOkxAnnouncements returns Record<string, unknown>, not typed
      // Just verify it returns something
      expect(Array.isArray(result)).toBe(true)
      expect(result).toHaveLength(1)
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('get_okx_announcements', 'Fetch failed')

      await expect(getOkxAnnouncements()).rejects.toThrow('Fetch failed')
    })
  })

  // -----------------------------------------------------------------------
  // 9. executeOkxOrder
  // -----------------------------------------------------------------------
  describe('executeOkxOrder', () => {
    it('returns order ID on success', async () => {
      const order = mockOrder()
      mockTauriInvoke('execute_okx_order', 'exec-12345')

      const result = await executeOkxOrder(order)

      expect(result).toBe('exec-12345')
    })

    it('passes order to Tauri command', async () => {
      const order = mockOrder({ order_id: 42 })
      mockTauriInvoke('execute_okx_order', 'exec-42')

      await executeOkxOrder(order)

      expect(mockInvoke).toHaveBeenCalledWith('execute_okx_order', {
        order,
      })
    })

    it('rejects on error', async () => {
      const order = mockOrder()
      mockTauriInvokeError('execute_okx_order', 'Execution failed')

      await expect(executeOkxOrder(order)).rejects.toThrow('Execution failed')
    })
  })

  // -----------------------------------------------------------------------
  // 10. getOkxRealtimeData
  // -----------------------------------------------------------------------
  describe('getOkxRealtimeData', () => {
    it('returns MarketData on success', async () => {
      const mockData = mockMarketData({ symbol: 'BTC-USDT', close: 50500 })
      mockTauriInvoke('get_okx_realtime_data', mockData)

      const result = await getOkxRealtimeData('BTC-USDT')

      expect(result).toHaveProperty('symbol', 'BTC-USDT')
      expect(result).toHaveProperty('close', 50500)
      expect(result).toHaveProperty('bid_prices')
      expect(result).toHaveProperty('ask_prices')
    })

    it('passes symbol to Tauri command', async () => {
      mockTauriInvoke('get_okx_realtime_data', mockMarketData())

      await getOkxRealtimeData('ETH-USDT')

      expect(mockInvoke).toHaveBeenCalledWith('get_okx_realtime_data', {
        symbol: 'ETH-USDT',
      })
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('get_okx_realtime_data', 'Data unavailable')

      await expect(getOkxRealtimeData('BTC-USDT')).rejects.toThrow(
        'Data unavailable',
      )
    })
  })

  // -----------------------------------------------------------------------
  // 11. getOkxHistoricalData
  // -----------------------------------------------------------------------
  describe('getOkxHistoricalData', () => {
    it('returns MarketData[] with date params', async () => {
      const mockData = [
        mockMarketData({ symbol: 'BTC-USDT', close: 50000 }),
        mockMarketData({ symbol: 'BTC-USDT', close: 50100 }),
      ]
      mockTauriInvoke('get_okx_historical_data', mockData)

      const result = await getOkxHistoricalData(
        'BTC-USDT',
        '2025-01-01',
        '2025-01-31',
      )

      expect(result).toHaveLength(2)
      expect(result[0]).toHaveProperty('symbol', 'BTC-USDT')
      expect(result[0]).toHaveProperty('close', 50000)
    })

    it('passes correct arguments to Tauri command', async () => {
      mockTauriInvoke('get_okx_historical_data', [])

      await getOkxHistoricalData('BTC-USDT', '2025-01-01', '2025-01-31')

      expect(mockInvoke).toHaveBeenCalledWith('get_okx_historical_data', {
        symbol: 'BTC-USDT',
        start: '2025-01-01',
        end: '2025-01-31',
      })
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('get_okx_historical_data', 'History error')

      await expect(
        getOkxHistoricalData('BTC-USDT', '2025-01-01', '2025-01-31'),
      ).rejects.toThrow('History error')
    })
  })

  // -----------------------------------------------------------------------
  // 12. subscribeMarketData
  // -----------------------------------------------------------------------
  describe('subscribeMarketData', () => {
    it('calls invoke with channel and symbol', async () => {
      mockTauriInvoke('subscribe_market_data', undefined)

      await subscribeMarketData('tickers', 'BTC-USDT')

      expect(mockInvoke).toHaveBeenCalledWith('subscribe_market_data', {
        channel: 'tickers',
        symbol: 'BTC-USDT',
      })
    })

    it('resolves without error', async () => {
      mockTauriInvoke('subscribe_market_data', undefined)

      await expect(
        subscribeMarketData('tickers', 'BTC-USDT'),
      ).resolves.toBeUndefined()
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('subscribe_market_data', 'Subscribe failed')

      await expect(
        subscribeMarketData('tickers', 'BTC-USDT'),
      ).rejects.toThrow('Subscribe failed')
    })
  })

  // -----------------------------------------------------------------------
  // 13. stopMarketData
  // -----------------------------------------------------------------------
  describe('stopMarketData', () => {
    it('calls invoke with command name', async () => {
      mockTauriInvoke('stop_market_data', undefined)

      await stopMarketData()

      expect(mockInvoke).toHaveBeenCalledWith('stop_market_data')
    })

    it('resolves without error', async () => {
      mockTauriInvoke('stop_market_data', undefined)

      await expect(stopMarketData()).resolves.toBeUndefined()
    })

    it('rejects on error', async () => {
      mockTauriInvokeError('stop_market_data', 'Stop failed')

      await expect(stopMarketData()).rejects.toThrow('Stop failed')
    })
  })

  // -----------------------------------------------------------------------
  // Type boundary: Rust Decimal → JSON → TypeScript number precision
  // -----------------------------------------------------------------------
  describe('large number precision (Rust Decimal boundary)', () => {
    it('handles large number values up to ~2^53-1 from Rust Decimal', async () => {
      const largeBalance = mockLargeNumberBalance()
      mockTauriInvoke('get_okx_balance', [largeBalance])

      const result = await getOkxBalance()

      // Number values up to 2^53-1 (MAX_SAFE_INTEGER) are preserved by JSON
      expect(result[0].ccy).toBe('BTC')
      expect(result[0].eq).toBe(largeBalance.eq)
      expect(result[0].cashBal).toBe(largeBalance.cashBal)
      // Verify it's within safe integer range
      expect(Number.isSafeInteger(result[0].eq)).toBe(true)
    })

    it('preserves decimal precision for fractional balances', async () => {
      const preciseBalance: OkxBalance = {
        ccy: 'ETH',
        cashBal: 0.123456789,
        frozenBal: 0.000000001,
        eq: 0.12345779,
        availEq: 0.12345779,
      }
      mockTauriInvoke('get_okx_balance', [preciseBalance])

      const result = await getOkxBalance()

      // Standard JSON/JS number precision is sufficient for typical crypto balances
      expect(result[0].cashBal).toBeCloseTo(0.123456789, 9)
      expect(result[0].frozenBal).toBeCloseTo(0.000000001, 9)
    })
  })
})
