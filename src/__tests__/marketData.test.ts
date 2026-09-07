import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import { useMarketDataStore, DEFAULT_MARKET_SYMBOLS } from '@/stores/marketData'
import { mockInvoke, mockTauriInvoke } from './mock-tauri'

type AnyHandler = (ev: { payload: unknown }) => void
let capturedHandlers: Record<string, AnyHandler> = {}
let activeListeners: Map<string, Set<AnyHandler>> = new Map()

function fireEvent(event: string, payload: unknown): void {
  const handler = capturedHandlers[event]
  if (!handler) throw new Error(`no listener registered for ${event}`)
  handler({ payload })
}

function captureListen(): void {
  vi.mocked(listen).mockImplementation((async (event: string, handler: AnyHandler) => {
    capturedHandlers[event] = handler
    const listeners = activeListeners.get(event) ?? new Set<AnyHandler>()
    listeners.add(handler)
    activeListeners.set(event, listeners)
    return () => {
      listeners.delete(handler)
      if (capturedHandlers[event] === handler) delete capturedHandlers[event]
    }
  }) as typeof listen)
}


function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void
  const promise = new Promise<T>((done) => { resolve = done })
  return { promise, resolve }
}

describe('marketData store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    capturedHandlers = {}
    activeListeners = new Map()
    vi.useFakeTimers()
    vi.clearAllMocks()
    mockInvoke.mockResolvedValue(undefined)
    captureListen()
  })

  afterEach(async () => {
    mockInvoke.mockResolvedValue(undefined)
    await useMarketDataStore().stop()
    vi.useRealTimers()
  })

  it('starts the stream, subscribes streams, and flips running', async () => {
    const store = useMarketDataStore()
    await store.start()

    expect(store.running).toBe(true)
    expect(store.status).toBe('connected')
    expect(store.symbols).toEqual(DEFAULT_MARKET_SYMBOLS)
    expect(store.activeSymbol).toBe('BTC-USDT')

    // 概览只订阅轻量 ticker；重流(trade/orderbook/candle)仅活跃标的。
    for (const sym of DEFAULT_MARKET_SYMBOLS) {
      expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_ticker', { symbol: sym })
    }
    const active = DEFAULT_MARKET_SYMBOLS[0]
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_trades', { symbol: active })
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_orderbook', { symbol: active })
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_candle', { symbol: active, interval: '1h' })
  })

  it('loads latest ticker from DB (get_ticker_snapshots)', async () => {
    mockTauriInvoke('get_ticker_snapshots', [
      {
        instrument_id: 'BTC-USDT',
        ts: '2024-01-01T00:00:00Z',
        last_px: 50000,
        open_24h: 49500,
        high_24h: 51000,
        low_24h: 49000,
        vol_24h: 1200,
        vol_ccy_24h: 60000000,
        change_24h: 500,
        created_at: null,
      },
    ])
    const store = useMarketDataStore()
    await store.start()

    expect(store.tickers['BTC-USDT']).toBeTruthy()
    expect(store.tickers['BTC-USDT'].last_price).toBe(50000)
    expect(store.tickers['BTC-USDT'].price_change_percent).toBeCloseTo((500 / 49500) * 100)
    expect(store.tickerList.length).toBeGreaterThanOrEqual(1)
  })

  it('loads trades from DB (get_trades)', async () => {
    mockTauriInvoke('get_trades', [
      { id: 1, symbol: 'BTC-USDT', price: 50000, quantity: 0.5, trade_time: '2024-01-01T00:00:00Z', is_buyer_maker: false, created_at: null },
      { id: 2, symbol: 'BTC-USDT', price: 50005, quantity: 0.6, trade_time: '2024-01-01T00:00:01Z', is_buyer_maker: false, created_at: null },
    ])
    const store = useMarketDataStore()
    await store.start()

    const list = store.trades['BTC-USDT']
    expect(list).toHaveLength(2)
    // pollDb 只拉活跃标的；DB 查询按 trade_time DESC，最新在前。
    expect(list[0].price).toBe(50000)
    expect(store.tradesForActive).toEqual(list)
  })

  it('loads orderbook from DB (get_orderbook)', async () => {
    mockTauriInvoke('get_orderbook', {
      symbol: 'BTC-USDT',
      bids: '[[50001,1],[50000,2]]',
      asks: '[[50002,1],[50003,2]]',
      ts: '2024-01-01T00:00:00Z',
      created_at: null,
    })
    const store = useMarketDataStore()
    await store.start()

    expect(store.orderBooks['BTC-USDT']).toBeTruthy()
    expect(store.orderBooks['BTC-USDT'].bids[0][0]).toBe(50001)
    expect(store.orderBooks['BTC-USDT'].asks[0][0]).toBe(50002)
    expect(store.orderBookForActive).toEqual(store.orderBooks['BTC-USDT'])
  })

  it('loads candles from DB (get_klines)', async () => {
    mockTauriInvoke('get_klines', [
      { id: 1, instrument_id: 'BTC-USDT', timeframe: '1h', timestamp: '2024-01-01T00:00:00Z', open: 50000, high: 50050, low: 49900, close: 50020, volume: 1200, created_at: null },
      { id: 2, instrument_id: 'BTC-USDT', timeframe: '1h', timestamp: '2024-01-01T01:00:00Z', open: 50020, high: 50100, low: 50010, close: 50050, volume: 800, created_at: null },
    ])
    const store = useMarketDataStore()
    await store.start()

    expect(store.candles['BTC-USDT']).toHaveLength(2)
    expect(store.candles['BTC-USDT'][0].close).toBe(50020)
    expect(store.candlesForActive).toHaveLength(2)
  })

  it('tracks connection status from binance:status', async () => {
    const store = useMarketDataStore()
    await store.start()

    fireEvent('binance:status', { status: 'connecting' })
    expect(store.status).toBe('connecting')

    fireEvent('binance:error', 'boom')
    expect(store.status).toBe('error')
  })

  it('stop detaches listeners and resets running', async () => {
    const store = useMarketDataStore()
    await store.start()
    await store.stop()
    expect(store.running).toBe(false)
    expect(store.status).toBe('idle')
    expect([...activeListeners.values()].every((listeners) => listeners.size === 0)).toBe(true)
    expect(vi.getTimerCount()).toBe(0)
  })

  it.each(['listen', 'subscribe'])('rolls back failed %s setup and restarts without duplicate listeners', async (failurePoint) => {
    const store = useMarketDataStore()
    if (failurePoint === 'listen') {
      const register = vi.mocked(listen).getMockImplementation()!
      vi.mocked(listen).mockImplementation((async (event: string, handler: AnyHandler) => {
        if (event === 'binance:orderbook') throw new Error('listen failed')
        return register(event, handler)
      }) as typeof listen)
    } else {
      mockInvoke.mockImplementation(async (command) => {
        if (command === 'subscribe_binance_ticker') throw new Error('subscription failed')
        return undefined
      })
    }

    await store.start()
    expect(store.running).toBe(false)
    expect(store.status).toBe('error')
    expect([...activeListeners.values()].every((listeners) => listeners.size === 0)).toBe(true)
    expect(vi.getTimerCount()).toBe(0)
    expect(mockInvoke).toHaveBeenCalledWith('stop_binance_market_data')

    captureListen()
    mockInvoke.mockResolvedValue(undefined)
    await store.start()
    expect(store.running).toBe(true)
    expect([...activeListeners.values()].every((listeners) => listeners.size === 1)).toBe(true)
    expect(vi.getTimerCount()).toBe(1)
  })

  it.each(['backend', 'listen', 'prefetch'])('stop during pending %s startup prevents late resurrection', async (stage) => {
    const entered = deferred<void>()
    const release = deferred<void>()
    if (stage === 'listen') {
      const register = vi.mocked(listen).getMockImplementation()!
      vi.mocked(listen).mockImplementation((async (event: string, handler: AnyHandler) => {
        if (event === 'binance:trade') {
          entered.resolve()
          await release.promise
        }
        return register(event, handler)
      }) as typeof listen)
    } else {
      mockInvoke.mockImplementation(async (command) => {
        if (command === (stage === 'backend' ? 'start_binance_market_data' : 'get_klines')) {
          entered.resolve()
          await release.promise
        }
        return []
      })
    }
    const store = useMarketDataStore()
    const startup = store.start()
    await entered.promise
    const stopping = store.stop()
    release.resolve()
    await Promise.all([startup, stopping])

    expect(store.running).toBe(false)
    expect(store.status).toBe('idle')
    expect(store.candles).toEqual({})
    expect([...activeListeners.values()].every((listeners) => listeners.size === 0)).toBe(true)
    expect(vi.getTimerCount()).toBe(0)

    captureListen()
    mockInvoke.mockResolvedValue(undefined)
    await store.start()
    expect(store.running).toBe(true)
    expect([...activeListeners.values()].every((listeners) => listeners.size === 1)).toBe(true)
  })

  it('a later stop cancels a restart queued behind shutdown', async () => {
    const store = useMarketDataStore()
    await store.start()
    const pendingStop = deferred<void>()
    mockInvoke.mockImplementation(async (command) => {
      if (command === 'stop_binance_market_data') return pendingStop.promise
      return []
    })
    const firstStop = store.stop()
    const restart = store.start()
    const finalStop = store.stop()
    pendingStop.resolve()
    await Promise.all([firstStop, restart, finalStop])
    expect(store.running).toBe(false)
    expect(store.status).toBe('idle')
    expect(vi.getTimerCount()).toBe(0)
    expect([...activeListeners.values()].every((listeners) => listeners.size === 0)).toBe(true)
  })

  it('releases local resources even when stopping the backend fails', async () => {
    const store = useMarketDataStore()
    await store.start()
    const lateStatus = capturedHandlers['binance:status']
    mockInvoke.mockImplementation(async (command) => {
      if (command === 'stop_binance_market_data') throw new Error('stop failed')
      return undefined
    })

    await expect(store.stop()).rejects.toThrow('stop failed')
    lateStatus({ payload: { status: 'connected' } })
    expect(store.running).toBe(false)
    expect(store.status).toBe('idle')
    expect(store.symbols).toEqual([])
    expect([...activeListeners.values()].every((listeners) => listeners.size === 0)).toBe(true)
    expect(vi.getTimerCount()).toBe(0)
    await expect(store.stop()).resolves.toBeUndefined()
  })

  it('does not refill cleared data from an in-flight poll after stop', async () => {
    mockInvoke.mockResolvedValue([])
    const store = useMarketDataStore()
    await store.start()
    const pendingRows = deferred<unknown[]>()
    mockInvoke.mockImplementation(async (command) => {
      if (command === 'get_klines') return pendingRows.promise
      return []
    })
    vi.advanceTimersByTime(5000)
    await store.stop()
    pendingRows.resolve([
      { instrument_id: 'BTC-USDT', timeframe: '1h', timestamp: '2024-01-01T00:00:00Z', open: 1, high: 2, low: 1, close: 2, volume: 1 },
    ])
    await Promise.resolve()
    await Promise.resolve()
    expect(store.candles).toEqual({})
  })
})
