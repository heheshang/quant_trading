import { describe, it, expect, beforeEach, vi } from 'vitest'
import type { WsTicker, WsTrade, WsOrderBook, WsCandle } from '../services/types'

// Module-level singleton state requires fresh module per test
let useMarketData: typeof import('../composables/useMarketData').useMarketData
let mockListen: ReturnType<typeof vi.fn>

beforeEach(async () => {
  vi.resetModules()
  const setup = await import('./setup')
  mockListen = setup.mockListen
  const mod = await import('../composables/useMarketData')
  useMarketData = mod.useMarketData
})

describe('useMarketData', () => {
  describe('returned shape', () => {
    it('returns tickerData, trades, orderbook, candleData, startListening, cleanup', () => {
      const market = useMarketData()

      expect(market).toHaveProperty('tickerData')
      expect(market).toHaveProperty('trades')
      expect(market).toHaveProperty('orderbook')
      expect(market).toHaveProperty('candleData')
      expect(market).toHaveProperty('startListening')
      expect(market).toHaveProperty('cleanup')

      expect(typeof market.startListening).toBe('function')
      expect(typeof market.cleanup).toBe('function')
    })
  })

  describe('startListening()', () => {
    it('registers 4 WS listeners (ticker, trades, orderbook, candle)', async () => {
      const { startListening } = useMarketData()

      await startListening()

      expect(mockListen).toHaveBeenCalledTimes(4)
      expect(mockListen).toHaveBeenCalledWith('ws:ticker', expect.any(Function))
      expect(mockListen).toHaveBeenCalledWith('ws:trades', expect.any(Function))
      expect(mockListen).toHaveBeenCalledWith('ws:orderbook', expect.any(Function))
      expect(mockListen).toHaveBeenCalledWith('ws:candle', expect.any(Function))
    })

    it('is idempotent — second call does not register more listeners', async () => {
      const { startListening } = useMarketData()

      await startListening()
      await startListening()

      expect(mockListen).toHaveBeenCalledTimes(4)
    })
  })

  describe('cleanup()', () => {
    it('calls all unlisten functions registered by startListening', async () => {
      const unlisten1 = vi.fn()
      const unlisten2 = vi.fn()
      const unlisten3 = vi.fn()
      const unlisten4 = vi.fn()
      mockListen
        .mockResolvedValueOnce(unlisten1)
        .mockResolvedValueOnce(unlisten2)
        .mockResolvedValueOnce(unlisten3)
        .mockResolvedValueOnce(unlisten4)

      const { startListening, cleanup } = useMarketData()
      await startListening()
      cleanup()

      expect(unlisten1).toHaveBeenCalledOnce()
      expect(unlisten2).toHaveBeenCalledOnce()
      expect(unlisten3).toHaveBeenCalledOnce()
      expect(unlisten4).toHaveBeenCalledOnce()
    })

    it('resets isListening so startListening can be called again afterward', async () => {
      const { startListening, cleanup } = useMarketData()

      await startListening()
      cleanup()
      await startListening()

      // 4 from first startListening + 4 from second = 8 total
      expect(mockListen).toHaveBeenCalledTimes(8)
    })
  })

  describe('event handling', () => {
    /**
     * Retrieves the callback registered for a given WS event name.
     * Must be called after startListening() has registered the listener.
     */
    function getCallback(eventName: string): (event: { payload: unknown }) => void {
      const call = mockListen.mock.calls.find(
        (c: unknown[]) => c[0] === eventName,
      )
      if (!call) {
        throw new Error(`No listener registered for ${eventName}`)
      }
      return call[1] as (event: { payload: unknown }) => void
    }

    describe('ws:ticker', () => {
      const makeTicker = (overrides: Partial<WsTicker> = {}): WsTicker => ({
        inst_id: 'BTC-USDT',
        last: '50000',
        last_sz: '1',
        ask_px: '50001',
        ask_sz: '2',
        bid_px: '49999',
        bid_sz: '3',
        open24h: '49000',
        high24h: '51000',
        low24h: '48000',
        vol24h: '1000',
        ts: '1234567890',
        ...overrides,
      })

      it('updates tickerData with incoming ticker', async () => {
        const { startListening, tickerData } = useMarketData()
        await startListening()

        const callback = getCallback('ws:ticker')
        const ticker = makeTicker()
        callback({ payload: ticker })

        expect(tickerData.value).toEqual({ 'BTC-USDT': ticker })
      })

      it('preserves existing ticker data when a new instrument arrives', async () => {
        const { startListening, tickerData } = useMarketData()
        await startListening()

        const callback = getCallback('ws:ticker')
        const btc = makeTicker({ inst_id: 'BTC-USDT', last: '50000' })
        const eth = makeTicker({ inst_id: 'ETH-USDT', last: '3000' })

        callback({ payload: btc })
        callback({ payload: eth })

        expect(tickerData.value).toEqual({
          'BTC-USDT': btc,
          'ETH-USDT': eth,
        })
      })

      it('overwrites existing ticker for the same inst_id', async () => {
        const { startListening, tickerData } = useMarketData()
        await startListening()

        const callback = getCallback('ws:ticker')
        const first = makeTicker({ inst_id: 'BTC-USDT', last: '50000' })
        const second = makeTicker({ inst_id: 'BTC-USDT', last: '51000' })

        callback({ payload: first })
        callback({ payload: second })

        expect(tickerData.value).toEqual({ 'BTC-USDT': second })
      })
    })

    describe('ws:trades', () => {
      const makeTrade = (overrides: Partial<WsTrade> = {}): WsTrade => ({
        inst_id: 'BTC-USDT',
        px: '50000',
        sz: '1',
        side: 'buy',
        ts: '1',
        ...overrides,
      })

      it('appends trades for each inst_id across batches', async () => {
        const { startListening, trades } = useMarketData()
        await startListening()

        const callback = getCallback('ws:trades')
        const t1 = makeTrade({ px: '50000', ts: '1' })
        const t2 = makeTrade({ px: '50001', ts: '2' })

        callback({ payload: [t1] })
        callback({ payload: [t2] })

        expect(trades.value['BTC-USDT']).toEqual([t1, t2])
      })

      it('respects MAX_TRADES_PER_SYMBOL (500) — discards oldest', async () => {
        const { startListening, trades } = useMarketData()
        await startListening()

        const callback = getCallback('ws:trades')
        const manyTrades: WsTrade[] = Array.from({ length: 501 }, (_, i) => ({
          inst_id: 'BTC-USDT',
          px: String(50000 + i),
          sz: '1',
          side: 'buy',
          ts: String(i),
        }))

        callback({ payload: manyTrades })

        const result = trades.value['BTC-USDT']
        expect(result).toHaveLength(500)
        // Kept the last 500: trades at indices 1..500
        expect(result[0].px).toBe('50001')
        expect(result[499].px).toBe('50500')
      })

      it('handles trades for multiple instruments in one batch', async () => {
        const { startListening, trades } = useMarketData()
        await startListening()

        const callback = getCallback('ws:trades')
        const btc = makeTrade({ inst_id: 'BTC-USDT', px: '50000' })
        const eth = makeTrade({ inst_id: 'ETH-USDT', px: '3000' })

        callback({ payload: [btc, eth] })

        expect(trades.value['BTC-USDT']).toEqual([btc])
        expect(trades.value['ETH-USDT']).toEqual([eth])
      })
    })

    describe('ws:orderbook', () => {
      const makeOrderbook = (
        overrides: Partial<WsOrderBook> = {},
      ): WsOrderBook => ({
        inst_id: 'BTC-USDT',
        asks: [
          ['50001', '2'],
          ['50002', '3'],
        ],
        bids: [
          ['49999', '1'],
          ['49998', '4'],
        ],
        ts: '1234567890',
        ...overrides,
      })

      it('updates orderbookData with incoming orderbook', async () => {
        const { startListening, orderbook } = useMarketData()
        await startListening()

        const callback = getCallback('ws:orderbook')
        const ob = makeOrderbook()
        callback({ payload: ob })

        expect(orderbook.value).toEqual({ 'BTC-USDT': ob })
      })

      it('overwrites existing orderbook for the same inst_id', async () => {
        const { startListening, orderbook } = useMarketData()
        await startListening()

        const callback = getCallback('ws:orderbook')
        const first = makeOrderbook({ asks: [['50001', '2']] })
        const second = makeOrderbook({ asks: [['50100', '5']] })

        callback({ payload: first })
        callback({ payload: second })

        expect(orderbook.value).toEqual({ 'BTC-USDT': second })
      })
    })

    describe('ws:candle', () => {
      const makeCandle = (overrides: Partial<WsCandle> = {}): WsCandle => ({
        inst_id: 'BTC-USDT',
        o: '49000',
        h: '51000',
        l: '48000',
        c: '50000',
        vol: '1000',
        ts: '1234567890',
        ...overrides,
      })

      it('updates candleData with incoming candle', async () => {
        const { startListening, candleData } = useMarketData()
        await startListening()

        const callback = getCallback('ws:candle')
        const candle = makeCandle()
        callback({ payload: candle })

        expect(candleData.value).toEqual({ 'BTC-USDT': candle })
      })

      it('overwrites existing candle for the same inst_id', async () => {
        const { startListening, candleData } = useMarketData()
        await startListening()

        const callback = getCallback('ws:candle')
        const first = makeCandle({ c: '50000' })
        const second = makeCandle({ c: '51000' })

        callback({ payload: first })
        callback({ payload: second })

        expect(candleData.value).toEqual({ 'BTC-USDT': second })
      })
    })
  })
})
