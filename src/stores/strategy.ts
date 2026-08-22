import { defineStore } from 'pinia'
import { ref, reactive, computed } from 'vue'
import {
  getStrategies,
  saveStrategy as apiSaveStrategy,
  deleteStrategy as apiDeleteStrategy,
  listStrategyTypes as apiListStrategyTypes,
  getStrategyTypeInfo as apiGetStrategyTypeInfo,
  createStrategy as apiCreateStrategy,
} from '@/services/strategy'
import type { StrategyParams, StrategyStatus, StrategyTypeInfo } from '@/services/types'

const POLL_INTERVAL_MS = 5_000

/**
 * Base strategy store (SRP): owns the strategy collection and its read /
 * create / update / delete / type-metadata operations.
 *
 * Lifecycle control (start/stop/pause/resume/deploy/archive/toggle) lives in
 * `useStrategyLifecycleStore`, which composes this store to refresh the
 * collection after a transition.
 */
type LoadingKey = 'list' | 'select' | 'create' | 'update' | 'delete' | 'listTypes' | 'fetchTypeInfo' | 'createNew'

type ErrorKey = LoadingKey

const LOADING_KEYS: LoadingKey[] = [
  'list',
  'select',
  'create',
  'update',
  'delete',
  'listTypes',
  'fetchTypeInfo',
  'createNew',
]

function makeLoadingRecord(): Record<LoadingKey, boolean> {
  return Object.fromEntries(LOADING_KEYS.map((k) => [k, false])) as Record<LoadingKey, boolean>
}

function makeErrorRecord(): Record<ErrorKey, string | null> {
  return Object.fromEntries(LOADING_KEYS.map((k) => [k, null])) as Record<ErrorKey, string | null>
}

export const useStrategyStore = defineStore('strategy', () => {
  const strategies = ref<StrategyParams[]>([])
  const currentStrategy = ref<StrategyParams | null>(null)
  const strategyTypes = ref<StrategyTypeInfo[]>([])
  const currentTypeInfo = ref<StrategyTypeInfo | null>(null)
  const loading = reactive<Record<LoadingKey, boolean>>(makeLoadingRecord())
  const error = reactive<Record<ErrorKey, string | null>>(makeErrorRecord())

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

  const isAnyLoading = computed(() => Object.values(loading).some((v) => v))

  async function fetchStrategies(force = false) {
    if (!force && strategies.value.length > 0) return
    loading.list = true
    error.list = null
    try {
      strategies.value = await getStrategies()
    } catch (err) {
      error.list = '获取策略列表失败'
      console.error('Failed to fetch strategies:', err)
    } finally {
      loading.list = false
    }
  }

  async function selectStrategy(id: string) {
    loading.select = true
    error.select = null
    try {
      const found = strategies.value.find((s) => s.strategy_id === id)
      if (found) {
        currentStrategy.value = found
      }
    } catch (err) {
      error.select = '获取策略详情失败'
      console.error('Failed to fetch strategy:', err)
    } finally {
      loading.select = false
    }
  }

  async function createStrategy(strategy: StrategyParams) {
    loading.create = true
    error.create = null
    try {
      await apiSaveStrategy(strategy)
      await fetchStrategies(true)
    } catch (err) {
      error.create = '创建策略失败'
      throw err
    } finally {
      loading.create = false
    }
  }

  async function updateStrategy(strategy: StrategyParams) {
    loading.update = true
    error.update = null
    try {
      await apiSaveStrategy(strategy)
      await fetchStrategies(true)
    } catch (err) {
      error.update = '更新策略失败'
      throw err
    } finally {
      loading.update = false
    }
  }

  async function deleteStrategy(strategyId: string) {
    loading.delete = true
    error.delete = null
    try {
      await apiDeleteStrategy(strategyId)
      await fetchStrategies(true)
    } catch (err) {
      error.delete = '删除策略失败'
      throw err
    } finally {
      loading.delete = false
    }
  }

  async function listStrategyTypes() {
    loading.listTypes = true
    error.listTypes = null
    try {
      strategyTypes.value = await apiListStrategyTypes()
    } catch (err) {
      error.listTypes = '获取策略类型列表失败'
      console.error('Failed to fetch strategy types:', err)
    } finally {
      loading.listTypes = false
    }
  }

  async function fetchStrategyTypeInfo(typeName: string) {
    loading.fetchTypeInfo = true
    error.fetchTypeInfo = null
    try {
      currentTypeInfo.value = await apiGetStrategyTypeInfo(typeName)
    } catch (err) {
      error.fetchTypeInfo = '获取策略类型信息失败'
      console.error('Failed to fetch strategy type info:', err)
    } finally {
      loading.fetchTypeInfo = false
    }
  }

  async function createNewStrategy(
    typeName: string,
    strategyName: string,
    params: Record<string, unknown>,
    enabled: boolean,
    maxPosition: number,
    maxDailyLoss: number,
    userId: number,
    instanceLabel?: string,
    description?: string,
    tags?: string[],
    symbols?: string[],
  ) {
    loading.createNew = true
    error.createNew = null
    try {
      const id = await apiCreateStrategy(
        typeName, strategyName, params, enabled,
        maxPosition, maxDailyLoss, userId, instanceLabel,
        description, tags, symbols,
      )
      await fetchStrategies(true)
      return id
    } catch (err) {
      error.createNew = '创建策略失败'
      throw err
    } finally {
      loading.createNew = false
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
    strategyTypes,
    currentTypeInfo,
    loading,
    error,
    isAnyLoading,
    runningStrategies,
    draftStrategies,
    strategyById,
    fetchStrategies,
    selectStrategy,
    createStrategy,
    updateStrategy,
    deleteStrategy,
    listStrategyTypes,
    fetchStrategyTypeInfo,
    createNewStrategy,
    startPolling,
    stopPolling,
  }
})
