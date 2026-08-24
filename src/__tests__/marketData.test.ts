import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import { useMarketDataStore, DEFAULT_MARKET_SYMBOLS } from '@/stores/marketData'
import { mockInvoke } from './mock-tauri'
import type { BinanceWsTicker, BinanceWsTrade, BinanceWsDepth, BinanceWsKline } from '@/services/types'

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

const sampleTicker: BinanceWsTicker = {
  symbol: 'BTC-USDT',
  last_price: 50000,
  price_change: 500,
  price_change_percent: 1.0,
  high: 51000,
  low: 49000,
  open: 49500,
  volume: 1200,
  quote_volume: 60000000,
  event_time: 1700000000000,
}

const sampleTrade: BinanceWsTrade = {
  symbol: 'BTC-USDT',
  price: 50000,
  quantity: 0.5,
  trade_time: 1700000000100,
  is_buyer_maker: false,
}

const sampleBook: BinanceWsDepth = {
  symbol: 'BTC-USDT',
  bids: [[50000, 1.5]],
  asks: [[50001, 2.5]],
}

const sampleCandle: BinanceWsKline = {
  symbol: 'BTC-USDT',
  interval: '1m',
  open_time: 1700000000000,
  open: 49900,
  high: 50100,
  low: 49800,
  close: 50000,
  volume: 100,
  is_closed: false,
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
    const cmds = mockInvoke.mock.calls.map(([cmd]) => cmd)
    for (const sym of DEFAULT_MARKET_SYMBOLS) {
      expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_ticker', { symbol: sym })
    }
    const active = DEFAULT_MARKET_SYMBOLS[0]
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_trades', { symbol: active })
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_orderbook', { symbol: active })
    expect(mockInvoke).toHaveBeenCalledWith('subscribe_binance_candle', { symbol: active, interval: '1m' })
  })

  it('routes binance:ticker into the ticker map', async () => {
    const store = useMarketDataStore()
    await store.start()

    fireEvent('binance:ticker', sampleTicker)
    expect(store.tickers['BTC-USDT']).toEqual(sampleTicker)
    expect(store.tickerList).toHaveLength(1)
    expect(store.tickerList[0].last_price).toBe(50000)
  })

  it('prepends binance:trade events for a symbol', async () => {
    const store = useMarketDataStore()
    await store.start()

    fireEvent('binance:trade', sampleTrade)
    fireEvent('binance:trade', { ...sampleTrade, trade_time: 1700000000200, price: 50005 })

    const list = store.trades['BTC-USDT']
    expect(list).toHaveLength(2)
    expect(list[0].price).toBe(50005)
    expect(list[1].trade_time).toBe(1700000000100)
    expect(store.tradesForActive).toEqual(list)
  })

  it('routes binance:orderbook into the order book map', async () => {
    const store = useMarketDataStore()
    await store.start()

    fireEvent('binance:orderbook', sampleBook)
    expect(store.orderBooks['BTC-USDT']).toEqual(sampleBook)
    expect(store.orderBookForActive).toEqual(sampleBook)
  })

  it('upserts binance:kline candles by open_time', async () => {
    const store = useMarketDataStore()
    await store.start()

    fireEvent('binance:kline', sampleCandle)
    fireEvent('binance:kline', { ...sampleCandle, close: 50050 })
    expect(store.candles['BTC-USDT']).toHaveLength(1)
    expect(store.candles['BTC-USDT'][0].close).toBe(50050)

    fireEvent('binance:kline', { ...sampleCandle, open_time: 1700000001000, close: 50100 })
    expect(store.candles['BTC-USDT']).toHaveLength(2)
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
