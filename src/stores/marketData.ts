import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import {
  listenToBinanceEvents,
  startBinanceStream,
  stopBinanceStream,
  type BinanceEventHandlers,
} from '@/services/binanceWS'
import {
  subscribeBinanceTicker,
  subscribeBinanceTrades,
  subscribeBinanceOrderbook,
  subscribeBinanceCandle,
  getBinanceCandles,
} from '@/services/binance'
import type {
  BinanceWsTicker,
  BinanceWsTrade,
  BinanceWsDepth,
  BinanceWsKline,
  BinanceKline,
} from '@/services/types'

/** Default symbols tracked by the realtime stream. */
export const DEFAULT_MARKET_SYMBOLS = ['BTC-USDT', 'ETH-USDT']
/** 概览 ticker 订阅上限（防大标的列表导致流数无界）。 */
export const MAX_SUBSCRIBE = 30

/**
 * Realtime Binance market data store.
 *
 * Owns the Binance WebSocket lifecycle (idempotent start) and translates the
 * `binance:ticker` / `binance:trade` / `binance:orderbook` / `binance:kline`
 * events into reactive state. Data is keyed per symbol so the ticker panel can
 * show several instruments while charts/order book focus on the active symbol.
 */
export const useMarketDataStore = defineStore('marketData', () => {
  const running = ref(false)
  const starting = ref(false)
  const status = ref('idle')
  const activeSymbol = ref(DEFAULT_MARKET_SYMBOLS[0])
  const symbols = ref<string[]>([])
  const tickers = ref<Record<string, BinanceWsTicker>>({})
  const trades = ref<Record<string, BinanceWsTrade[]>>({})
  const orderBooks = ref<Record<string, BinanceWsDepth>>({})
  const candles = ref<Record<string, BinanceWsKline[]>>({})

  let unlisteners: UnlistenFn[] = []
  let handlers: BinanceEventHandlers | null = null

  const tickerList = computed(() => Object.values(tickers.value))
  const tradesForActive = computed(() => trades.value[activeSymbol.value] ?? [])
  const orderBookForActive = computed(() => orderBooks.value[activeSymbol.value] ?? null)
  const candlesForActive = computed(() => candles.value[activeSymbol.value] ?? [])

  function upsertCandle(k: BinanceWsKline) {
    const list = candles.value[k.symbol] ?? []
    const idx = list.findIndex((c) => c.open_time === k.open_time)
    if (idx >= 0) {
      const next = [...list]
      next[idx] = k
      candles.value = { ...candles.value, [k.symbol]: next }
    } else {
      candles.value = { ...candles.value, [k.symbol]: [...list, k].slice(-120) }
    }
  }

  /** REST 历史 K 线 → WS K 线（首屏预取，避免 chart 全空等 WS）。 */
  function binanceKlineToWs(k: BinanceKline, symbol: string): BinanceWsKline {
    return {
      symbol,
      interval: '1m',
      open_time: k.open_time,
      open: k.open,
      high: k.high,
      low: k.low,
      close: k.close,
      volume: k.volume,
      is_closed: k.close_time <= Date.now(),
    }
  }

  /** 预取某标的近 100 根历史 1m K 线，先填 chart，WS 再实时增量更新。 */
  async function prefetchCandles(sym: string) {
    try {
      const rows = await getBinanceCandles(sym, '1m', 100)
      candles.value = {
        ...candles.value,
        [sym]: rows.map((k) => binanceKlineToWs(k, sym)),
      }
    } catch {
      // 预取失败忽略：等 WS 实时事件填充
    }
  }

  async function buildHandlers(): Promise<BinanceEventHandlers> {
    return {
      onTicker: (t) => {
        tickers.value = { ...tickers.value, [t.symbol]: t }
      },
      onTrade: (t) => {
        const list = [t, ...(trades.value[t.symbol] ?? [])].slice(0, 100)
        trades.value = { ...trades.value, [t.symbol]: list }
      },
      onOrderBook: (d) => {
        orderBooks.value = { ...orderBooks.value, [d.symbol]: d }
      },
      onCandle: upsertCandle,
      onStatus: (s) => {
        status.value = s.status
      },
      onError: () => {
        status.value = 'error'
      },
    }
  }

  /** 订阅某标的的重流（trade/orderbook/candle）——只给活跃标的用，防 N×4 洪泛。 */
  async function subscribeHeavy(sym: string): Promise<void> {
    await Promise.all([
      subscribeBinanceTrades(sym),
      subscribeBinanceOrderbook(sym),
      subscribeBinanceCandle(sym, '1m'),
    ])
  }

  async function ensureSubscribed(syms: string[]) {
    // 行情概览：只订阅轻量 ticker（每个标的一条流）；重流仅活跃标的订阅。
    // 上限 MAX_SUBSCRIBE 个标的，防止大标的列表导致流数无界（默认仅 2）。
    for (const sym of syms.slice(0, MAX_SUBSCRIBE)) {
      await subscribeBinanceTicker(sym)
    }
    symbols.value = Array.from(new Set([...symbols.value, ...syms]))
    await subscribeHeavy(activeSymbol.value)
  }

  /** Start the stream once (idempotent). Subsequent calls no-op. */
  async function start(syms: string[] = DEFAULT_MARKET_SYMBOLS) {
    if (running.value || starting.value) return
    starting.value = true
    try {
      await startBinanceStream()
      handlers = await buildHandlers()
      unlisteners = await listenToBinanceEvents(handlers)
      await ensureSubscribed(syms)
      activeSymbol.value = syms[0] ?? activeSymbol.value
      // 首屏预取历史 K 线，避免 chart 空白等 WS 首批事件。
      await prefetchCandles(activeSymbol.value)
      running.value = true
      status.value = 'connected'
    } catch {
      status.value = 'error'
    } finally {
      starting.value = false
    }
  }

  /** 清空按标的累积的实时数据（登出/切换用户时防旧数据残留）。 */
  function clear() {
    tickers.value = {}
    trades.value = {}
    orderBooks.value = {}
    candles.value = {}
    symbols.value = []
  }

  /** Stop the stream and detach all listeners. */
  async function stop() {
    if (!running.value) return
    unlisteners.forEach((u) => u())
    unlisteners = []
    await stopBinanceStream()
    running.value = false
    status.value = 'idle'
    clear()
  }

  function setActiveSymbol(sym: string) {
    if (symbols.value.includes(sym)) {
      activeSymbol.value = sym
      prefetchCandles(sym)
      // 切换标的时订阅其重流（后端去重，重复调用安全）。
      void subscribeHeavy(sym)
    }
  }

  return {
    running,
    status,
    activeSymbol,
    symbols,
    tickers,
    trades,
    orderBooks,
    candles,
    tickerList,
    tradesForActive,
    orderBookForActive,
    candlesForActive,
    start,
    stop,
    clear,
    setActiveSymbol,
  }
})
