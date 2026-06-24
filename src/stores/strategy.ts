import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  getStrategies,
  saveStrategy,
  deleteStrategy,
  toggleStrategy,
} from '@/services/api'
import type { StrategyParams } from '@/services/types'

const CACHE_TTL_MS = 30_000

export const useStrategyStore = defineStore('strategy', () => {
  const strategies = ref<StrategyParams[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  let lastFetched = 0

  const isStale = computed(() => Date.now() - lastFetched > CACHE_TTL_MS)

  const enabledStrategies = computed(() =>
    strategies.value.filter((s: StrategyParams) => s.enabled),
  )

  async function fetchStrategies(force = false) {
    if (!force && strategies.value.length > 0 && !isStale.value) return
    loading.value = true
    error.value = null
    try {
      strategies.value = await getStrategies()
      lastFetched = Date.now()
    } catch (err) {
      error.value = '获取策略列表失败'
      console.error('Failed to fetch strategies:', err)
    } finally {
      loading.value = false
    }
  }

  async function createStrategy(strategy: StrategyParams) {
    await saveStrategy(strategy)
    await fetchStrategies(true)
  }

  async function updateStrategy(strategy: StrategyParams) {
    await saveStrategy(strategy)
    await fetchStrategies(true)
  }

  async function removeStrategy(strategyId: string) {
    await deleteStrategy(strategyId)
    await fetchStrategies(true)
  }

  async function setStrategyEnabled(strategyId: string, enabled: boolean) {
    await toggleStrategy(strategyId, enabled)
    const found = strategies.value.find((s: StrategyParams) => s.strategy_id === strategyId)
    if (found) {
      found.enabled = enabled
    }
  }

  return {
    strategies,
    loading,
    error,
    isStale,
    enabledStrategies,
    fetchStrategies,
    createStrategy,
    updateStrategy,
    removeStrategy,
    setStrategyEnabled,
  }
})
