import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  getRiskMetrics,
  getRiskConfig,
  updateRiskConfig,
} from '@/services/risk'
import type { RiskConfig } from '@/services/types'

const CACHE_TTL_MS = 30_000

export const useRiskStore = defineStore('risk', () => {
  const metrics = ref<Record<string, number>>({})
  const config = ref<RiskConfig | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const lastFetchedMetrics = ref(0)
  const lastFetchedConfig = ref(0)
  let generation = 0

  const isStale = computed(
    () =>
      Date.now() - lastFetchedMetrics.value > CACHE_TTL_MS ||
      Date.now() - lastFetchedConfig.value > CACHE_TTL_MS,
  )

  const var95 = computed(() => metrics.value.var_95 ?? 0)
  const var99 = computed(() => metrics.value.var_99 ?? 0)
  const sharpeRatio = computed(() => metrics.value.sharpe_ratio ?? 0)
  const maxDrawdown = computed(() => metrics.value.max_drawdown ?? 0)
  const volatility = computed(() => metrics.value.volatility ?? 0)

  async function fetchMetrics(force = false) {
    if (!force && Object.keys(metrics.value).length > 0 && !isStale.value) return
    const requestGeneration = generation
    loading.value = true
    error.value = null
    try {
      const result = await getRiskMetrics()
      if (requestGeneration !== generation) return
      metrics.value = result
      lastFetchedMetrics.value = Date.now()
    } catch (err) {
      if (requestGeneration !== generation) return
      error.value = '获取风险指标失败'
      console.error('Failed to fetch risk metrics:', err)
    } finally {
      if (requestGeneration === generation) loading.value = false
    }
  }

  async function fetchConfig(force = false) {
    if (!force && config.value && !isStale.value) return
    const requestGeneration = generation
    try {
      const result = await getRiskConfig()
      if (requestGeneration !== generation) return
      config.value = result
      lastFetchedConfig.value = Date.now()
    } catch (err) {
      if (requestGeneration !== generation) return
      console.error('Failed to fetch risk config:', err)
    }
  }

  async function saveConfig(newConfig: RiskConfig) {
    const requestGeneration = generation
    try {
      await updateRiskConfig(newConfig)
      if (requestGeneration !== generation) return
      config.value = newConfig
    } catch (err) {
      if (requestGeneration !== generation) return
      error.value = '保存风控配置失败'
      console.error('Failed to update risk config:', err)
    }
  }

  async function refreshAll() {
    const requestGeneration = generation
    loading.value = true
    error.value = null
    try {
      const [m, c] = await Promise.all([getRiskMetrics(), getRiskConfig()])
      if (requestGeneration !== generation) return
      metrics.value = m
      config.value = c
      lastFetchedMetrics.value = Date.now()
      lastFetchedConfig.value = Date.now()
    } catch (err) {
      if (requestGeneration !== generation) return
      error.value = '刷新风控数据失败'
    } finally {
      if (requestGeneration === generation) loading.value = false
    }
  }

  function reset() {
    generation += 1
    metrics.value = {}
    config.value = null
    loading.value = false
    error.value = null
    lastFetchedMetrics.value = 0
    lastFetchedConfig.value = 0
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
    reset,
  }
})
