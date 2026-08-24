import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import { useMarketDataStore, DEFAULT_MARKET_SYMBOLS } from '@/stores/marketData'
import { mockInvoke, mockTauriInvoke } from './mock-tauri'

type AnyHandler = (ev: { payload: unknown }) => void
let capturedHandlers: Record<string, AnyHandler> = {}

function fireEvent(event: string, payload: unknown): void {
  const handler = capturedHandlers[event]
  if (!handler) throw new Error(`no listener registered for ${event}`)
  handler({ payload })
}

function captureListen(): void {
  vi.mocked(listen).mockImplementation((async (event: string, handler: AnyHandler) => {
    capturedHandlers[event] = handler
    return () => {}
  }) as typeof listen)
}





describe('marketData store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    capturedHandlers = {}
    vi.clearAllMocks()
    captureListen()
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
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_candle', { symbol: active, interval: '1m' })
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
      { id: 1, instrument_id: 'BTC-USDT', timeframe: '1m', timestamp: '2024-01-01T00:00:00Z', open: 50000, high: 50050, low: 49900, close: 50020, volume: 1200, created_at: null },
      { id: 2, instrument_id: 'BTC-USDT', timeframe: '1m', timestamp: '2024-01-01T00:01:00Z', open: 50020, high: 50100, low: 50010, close: 50050, volume: 800, created_at: null },
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
    const unlisteners: Array<() => void> = []
    unlisteners.push(() => {})

    await store.stop()
    expect(store.running).toBe(false)
    expect(store.status).toBe('idle')
  })
})
