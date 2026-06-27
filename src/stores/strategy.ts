import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  getStrategies,
  saveStrategy as apiSaveStrategy,
  deleteStrategy as apiDeleteStrategy,
  startStrategy as apiStartStrategy,
  stopStrategy as apiStopStrategy,
  pauseStrategy as apiPauseStrategy,
  resumeStrategy as apiResumeStrategy,
  deployStrategy as apiDeployStrategy,
  archiveStrategy as apiArchiveStrategy,
  toggleStrategy as apiToggleStrategy,
} from '@/services/strategy'
import type { StrategyParams, StrategyStatus } from '@/services/types'

const POLL_INTERVAL_MS = 5_000

export const useStrategyStore = defineStore('strategy', () => {
  const strategies = ref<StrategyParams[]>([])
  const currentStrategy = ref<StrategyParams | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  let pollTimer: ReturnType<typeof setInterval> | null = null

  const runningStrategies = computed(() =>
    strategies.value.filter((s) => {
      const status = s.status as StrategyStatus | undefined
      return status === 'Running'
    })
  )

  const draftStrategies = computed(() =>
    strategies.value.filter((s) => {
      const status = s.status as StrategyStatus | undefined
      return status === 'Draft' || status === undefined
    }),
  )

  const strategyById = computed(() => {
    return (id: string) => strategies.value.find((s) => s.strategy_id === id)
  })

  async function fetchStrategies(force = false) {
    if (!force && strategies.value.length > 0) return
    loading.value = true
    error.value = null
    try {
      strategies.value = await getStrategies()
    } catch (err) {
      error.value = '获取策略列表失败'
      console.error('Failed to fetch strategies:', err)
    } finally {
      loading.value = false
    }
  }

  async function fetchStrategy(id: string) {
    loading.value = true
    error.value = null
    try {
      const found = strategies.value.find((s) => s.strategy_id === id)
      if (found) {
        currentStrategy.value = found
      }
    } catch (err) {
      error.value = '获取策略详情失败'
      console.error('Failed to fetch strategy:', err)
    } finally {
      loading.value = false
    }
  }

  async function createStrategy(strategy: StrategyParams) {
    loading.value = true
    error.value = null
    try {
      await apiSaveStrategy(strategy)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '创建策略失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function updateStrategy(strategy: StrategyParams) {
    loading.value = true
    error.value = null
    try {
      await apiSaveStrategy(strategy)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '更新策略失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function deleteStrategy(strategyId: string) {
    loading.value = true
    error.value = null
    try {
      await apiDeleteStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '删除策略失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  async function toggleStrategy(strategyId: string, enabled: boolean) {
    error.value = null
    try {
      await apiToggleStrategy(strategyId, enabled)
      const found = strategies.value.find((s) => s.strategy_id === strategyId)
      if (found) {
        found.enabled = enabled
      }
    } catch (err) {
      error.value = '更新策略状态失败'
      throw err
    }
  }

  async function startStrategy(strategyId: string) {
    error.value = null
    try {
      await apiStartStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '启动策略失败'
      throw err
    }
  }

  async function stopStrategy(strategyId: string) {
    error.value = null
    try {
      await apiStopStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '停止策略失败'
      throw err
    }
  }

  async function pauseStrategy(strategyId: string) {
    error.value = null
    try {
      await apiPauseStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '暂停策略失败'
      throw err
    }
  }

  async function resumeStrategy(strategyId: string) {
    error.value = null
    try {
      await apiResumeStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '恢复策略失败'
      throw err
    }
  }

  async function deployStrategy(strategyId: string) {
    error.value = null
    try {
      await apiDeployStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '部署策略失败'
      throw err
    }
  }

  async function archiveStrategy(strategyId: string) {
    error.value = null
    try {
      await apiArchiveStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.value = '归档策略失败'
      throw err
    }
  }

  function startPolling() {
    if (pollTimer) return
    pollTimer = setInterval(() => {
      fetchStrategies(true)
    }, POLL_INTERVAL_MS)
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  return {
    strategies,
    currentStrategy,
    loading,
    error,
    runningStrategies,
    draftStrategies,
    strategyById,
    fetchStrategies,
    fetchStrategy,
    createStrategy,
    updateStrategy,
    deleteStrategy,
    toggleStrategy,
    startStrategy,
    stopStrategy,
    pauseStrategy,
    resumeStrategy,
    deployStrategy,
    archiveStrategy,
    startPolling,
    stopPolling,
  }
})
