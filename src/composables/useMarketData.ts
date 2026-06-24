import { shallowRef, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { WsTicker, WsTrade, WsOrderBook, WsCandle } from '@/services/types'

const MAX_TRADES_PER_SYMBOL = 500

// --- Module-level singleton state ---
const tickerData = shallowRef<Record<string, WsTicker>>({})
const trades = shallowRef<Record<string, WsTrade[]>>({})
const orderbook = shallowRef<Record<string, WsOrderBook>>({})
const candleData = shallowRef<Record<string, WsCandle>>({})

let unlisteners: UnlistenFn[] = []
let isListening = false

async function startListening() {
  if (isListening) return
  isListening = true

  const tickerUnlisten = await listen<WsTicker>('ws:ticker', (event) => {
    const data = event.payload
    tickerData.value = { ...tickerData.value, [data.inst_id]: data }
  })
  unlisteners.push(tickerUnlisten)

  const tradesUnlisten = await listen<WsTrade[]>('ws:trades', (event) => {
    const incoming = event.payload
    for (const trade of incoming) {
      const existing = trades.value[trade.inst_id] ?? []
      const updated = [...existing, trade]
      if (updated.length > MAX_TRADES_PER_SYMBOL) {
        updated.splice(0, updated.length - MAX_TRADES_PER_SYMBOL)
      }
      trades.value = { ...trades.value, [trade.inst_id]: updated }
    }
  })
  unlisteners.push(tradesUnlisten)

  const orderbookUnlisten = await listen<WsOrderBook>('ws:orderbook', (event) => {
    const data = event.payload
    orderbook.value = { ...orderbook.value, [data.inst_id]: data }
  })
  unlisteners.push(orderbookUnlisten)

  const candleUnlisten = await listen<WsCandle>('ws:candle', (event) => {
    const data = event.payload
    candleData.value = { ...candleData.value, [data.inst_id]: data }
  })
  unlisteners.push(candleUnlisten)
}

function cleanup() {
  for (const unlisten of unlisteners) {
    unlisten()
  }
  unlisteners = []
  isListening = false
}

export function useMarketData() {
  return {
    tickerData: computed(() => tickerData.value),
    trades: computed(() => trades.value),
    orderbook: computed(() => orderbook.value),
    candleData: computed(() => candleData.value),
    startListening,
    cleanup,
  }
}
