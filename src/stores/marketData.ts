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
} from '@/services/binance'
import { getKlines, getTickerSnapshots } from '@/services/market'
import type {
  BinanceWsTicker,
  BinanceWsTrade,
  BinanceWsDepth,
  BinanceWsKline,
  MarketDataRecord,
  TickerSnapshotRecord,
} from '@/services/types'

/** Default symbols tracked by the realtime stream. */
export const DEFAULT_MARKET_SYMBOLS = ['BTC-USDT', 'ETH-USDT']
/** 概览 ticker 订阅上限（防大标的列表导致流数无界）。 */
export const MAX_SUBSCRIBE = 30

/**
 * Binance market data store (DB-first).
 *
 * 行情源按「remote WS → DB 导入 → 前端读 DB」：K 线 / ticker 经 DB 轮询
 * （`get_klines` / `get_ticker_snapshots`）读取；逐笔成交与订单簿仍走
 * `binance:trade` / `binance:orderbook` 实时事件（暂无对应 DB 表）。
 * 数据按标的关键键，ticker 面板可展示多标的，图表/订单簿聚焦活跃标的。
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
  let dbPollTimer: ReturnType<typeof setInterval> | null = null

  const tickerList = computed(() => Object.values(tickers.value))
  const tradesForActive = computed(() => trades.value[activeSymbol.value] ?? [])
  const orderBookForActive = computed(() => orderBooks.value[activeSymbol.value] ?? null)
  const candlesForActive = computed(() => candles.value[activeSymbol.value] ?? [])

  /** DB K 线行 → chart 使用的 WS K 线（`timestamp`→`open_time`）。 */
  function dbKlineToWs(r: MarketDataRecord): BinanceWsKline {
    return {
      symbol: r.instrument_id,
      interval: r.timeframe,
      open_time: new Date(r.timestamp).getTime(),
      open: r.open,
      high: r.high,
      low: r.low,
      close: r.close,
      volume: r.volume,
      is_closed: true,
    }
  }

  /** DB ticker 快照行 → WS ticker（`change_24h` 为绝对变动，按 open 折算百分比）。 */
  function dbTickerToWs(r: TickerSnapshotRecord): BinanceWsTicker {
    const last = r.last_px ?? 0
    const open = r.open_24h ?? 0
    const change = r.change_24h ?? 0
    return {
      symbol: r.instrument_id,
      last_price: last,
      price_change: change,
      price_change_percent: open > 0 ? (change / open) * 100 : 0,
      high: r.high_24h ?? 0,
      low: r.low_24h ?? 0,
      open,
      volume: r.vol_24h ?? 0,
      quote_volume: r.vol_ccy_24h ?? 0,
      event_time: new Date(r.ts).getTime(),
    }
  }

  /** 预取某标的近 100 根历史 1m K 线（从数据库），先填 chart。 */
  async function prefetchCandles(sym: string) {
    try {
      const rows = await getKlines(sym, '1m', 100)
      candles.value = {
        ...candles.value,
        [sym]: rows.map(dbKlineToWs),
      }
    } catch {
      // 预取失败忽略：等 DB 轮询填充
    }
  }

  /** 从数据库拉取活跃标的 K 线 + 全部订阅标的的最新 ticker（纯 DB 读）。 */
  async function pollDb() {
    try {
      const rows = await getKlines(activeSymbol.value, '1m', 120)
      candles.value = { ...candles.value, [activeSymbol.value]: rows.map(dbKlineToWs) }
    } catch {
      // 忽略：下次轮询重试
    }
    const syms = symbols.value.length ? symbols.value : DEFAULT_MARKET_SYMBOLS
    for (const sym of syms.slice(0, MAX_SUBSCRIBE)) {
      try {
        const rows = await getTickerSnapshots(sym, 1)
        const last = rows[0]
        if (last) tickers.value = { ...tickers.value, [sym]: dbTickerToWs(last) }
      } catch {
        // 忽略单标的失败
      }
    }
  }

  async function buildHandlers(): Promise<BinanceEventHandlers> {
    return {
      // ticker / candle 改由 DB 轮询（pollDb）驱动，不经 WS 事件。
      onTrade: (t) => {
        const list = [t, ...(trades.value[t.symbol] ?? [])].slice(0, 100)
        trades.value = { ...trades.value, [t.symbol]: list }
      },
      onOrderBook: (d) => {
        orderBooks.value = { ...orderBooks.value, [d.symbol]: d }
      },
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
      // 首屏预取历史 K 线，避免 chart 空白等 DB 首批轮询。
      await prefetchCandles(activeSymbol.value)
      await pollDb()
      if (dbPollTimer) clearInterval(dbPollTimer)
      dbPollTimer = setInterval(pollDb, 5000)
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
    if (dbPollTimer) {
      clearInterval(dbPollTimer)
      dbPollTimer = null
    }
    await stopBinanceStream()
    running.value = false
    status.value = 'idle'
    clear()
  }

  function setActiveSymbol(sym: string) {
    if (symbols.value.includes(sym)) {
      activeSymbol.value = sym
      prefetchCandles(sym)
      void pollDb()
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
