import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  subscribeChannel,
  unsubscribeChannel,
  getSubscriptions,
} from '@/services/api'

const CHANNEL_OPTIONS = ['ticker', 'trades', 'orderbook', 'candle'] as const
export type Channel = (typeof CHANNEL_OPTIONS)[number]

export const useMarketDataStore = defineStore('marketData', () => {
  const subscriptions = ref<string[]>([])
  const activeSymbols = ref<string[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const subscriptionCount = computed(() => subscriptions.value.length)

  function hasSubscription(symbol: string, channel: string): boolean {
    const key = `${symbol}:${channel}`
    return subscriptions.value.includes(key)
  }

  async function subscribe(symbol: string, channel: string) {
    const key = `${symbol}:${channel}`
    if (hasSubscription(symbol, channel)) {
      error.value = `已订阅 ${key}，请勿重复操作`
      return
    }
    loading.value = true
    error.value = null
    try {
      await subscribeChannel(symbol, channel)
      subscriptions.value.push(key)
      if (!activeSymbols.value.includes(symbol)) {
        activeSymbols.value.push(symbol)
      }
    } catch (err) {
      error.value = `订阅 ${key} 失败`
      console.error('Failed to subscribe:', err)
    } finally {
      loading.value = false
    }
  }

  async function unsubscribe(symbol: string, channel: string) {
    const key = `${symbol}:${channel}`
    if (!hasSubscription(symbol, channel)) return
    loading.value = true
    error.value = null
    try {
      await unsubscribeChannel(symbol, channel)
      subscriptions.value = subscriptions.value.filter((s) => s !== key)
      const remaining = subscriptions.value.filter((s) => s.startsWith(`${symbol}:`))
      if (remaining.length === 0) {
        activeSymbols.value = activeSymbols.value.filter((s) => s !== symbol)
      }
    } catch (err) {
      error.value = `取消订阅 ${key} 失败`
      console.error('Failed to unsubscribe:', err)
    } finally {
      loading.value = false
    }
  }

  async function refreshSubscriptions() {
    loading.value = true
    error.value = null
    try {
      subscriptions.value = await getSubscriptions()
      const symbols = new Set<string>()
      for (const sub of subscriptions.value) {
        const [sym] = sub.split(':')
        if (sym) symbols.add(sym)
      }
      activeSymbols.value = Array.from(symbols)
    } catch (err) {
      error.value = '获取订阅列表失败'
      console.error('Failed to refresh subscriptions:', err)
    } finally {
      loading.value = false
    }
  }

  return {
    subscriptions,
    activeSymbols,
    loading,
    error,
    subscriptionCount,
    hasSubscription,
    subscribe,
    unsubscribe,
    refreshSubscriptions,
  }
})
