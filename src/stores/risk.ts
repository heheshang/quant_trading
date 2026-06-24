import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  getRiskMetrics,
  getRiskConfig,
  updateRiskConfig,
} from '@/services/api'
import type { RiskConfig } from '@/services/types'

const CACHE_TTL_MS = 30_000

export const useRiskStore = defineStore('risk', () => {
  const metrics = ref<Record<string, number>>({})
  const config = ref<RiskConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  let lastFetchedMetrics = 0
  let lastFetchedConfig = 0

  const isStale = computed(
    () =>
      Date.now() - lastFetchedMetrics > CACHE_TTL_MS ||
      Date.now() - lastFetchedConfig > CACHE_TTL_MS,
  )

  const var95 = computed(() => metrics.value.var_95 ?? 0)
  const var99 = computed(() => metrics.value.var_99 ?? 0)
  const sharpeRatio = computed(() => metrics.value.sharpe_ratio ?? 0)
  const maxDrawdown = computed(() => metrics.value.max_drawdown ?? 0)
  const volatility = computed(() => metrics.value.volatility ?? 0)

  async function fetchMetrics(force = false) {
    if (!force && Object.keys(metrics.value).length > 0 && !isStale.value) return
    loading.value = true
    error.value = null
    try {
      metrics.value = await getRiskMetrics()
      lastFetchedMetrics = Date.now()
    } catch (err) {
      error.value = '获取风险指标失败'
      console.error('Failed to fetch risk metrics:', err)
    } finally {
      loading.value = false
    }
  }

  async function fetchConfig(force = false) {
    if (!force && config.value && !isStale.value) return
    try {
      config.value = await getRiskConfig()
      lastFetchedConfig = Date.now()
    } catch (err) {
      console.error('Failed to fetch risk config:', err)
    }
  }

  async function saveConfig(newConfig: RiskConfig) {
    try {
      await updateRiskConfig(newConfig)
      config.value = newConfig
    } catch (err) {
      error.value = '保存风控配置失败'
      console.error('Failed to update risk config:', err)
    }
  }

  async function refreshAll() {
    loading.value = true
    error.value = null
    try {
      const [m, c] = await Promise.all([getRiskMetrics(), getRiskConfig()])
      metrics.value = m
      config.value = c
      lastFetchedMetrics = Date.now()
      lastFetchedConfig = Date.now()
    } catch (err) {
      error.value = '刷新风控数据失败'
    } finally {
      loading.value = false
    }
  }

  return {
    metrics,
    config,
    loading,
    error,
    isStale,
    var95,
    var99,
    sharpeRatio,
    maxDrawdown,
    volatility,
    fetchMetrics,
    fetchConfig,
    saveConfig,
    refreshAll,
  }
})
