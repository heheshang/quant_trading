import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { useStrategyStore } from './strategy'
import {
  startStrategy as apiStartStrategy,
  stopStrategy as apiStopStrategy,
  pauseStrategy as apiPauseStrategy,
  resumeStrategy as apiResumeStrategy,
  deployStrategy as apiDeployStrategy,
  archiveStrategy as apiArchiveStrategy,
  toggleStrategy as apiToggleStrategy,
} from '@/services/strategy'

/**
 * Strategy lifecycle store (SRP): owns runtime lifecycle transitions
 * (start / stop / pause / resume / deploy / archive / toggle) for a strategy.
 *
 * It composes the base `useStrategyStore` to refresh the collection after a
 * transition, and exposes its own per-action error state. Keeping this
 * separate from the base store decouples "control" from "data" concerns.
 */

type LifecycleKey = 'toggle' | 'start' | 'stop' | 'pause' | 'resume' | 'deploy' | 'archive'

const LIFECYCLE_KEYS: LifecycleKey[] = [
  'toggle',
  'start',
  'stop',
  'pause',
  'resume',
  'deploy',
  'archive',
]

function makeLifecycleError(): Record<LifecycleKey, string | null> {
  return Object.fromEntries(LIFECYCLE_KEYS.map((k) => [k, null])) as Record<LifecycleKey, string | null>
}

export const useStrategyLifecycleStore = defineStore('strategyLifecycle', () => {
  const strategyStore = useStrategyStore()
  const error = reactive<Record<LifecycleKey, string | null>>(makeLifecycleError())

  async function startStrategy(strategyId: string) {
    error.start = null
    try {
      await apiStartStrategy(strategyId)
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      error.start = '启动策略失败'
      throw err
    }
  }

  async function stopStrategy(strategyId: string) {
    error.stop = null
    try {
      await apiStopStrategy(strategyId)
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      error.stop = '停止策略失败'
      throw err
    }
  }

  async function pauseStrategy(strategyId: string) {
    error.pause = null
    try {
      await apiPauseStrategy(strategyId)
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      error.pause = '暂停策略失败'
      throw err
    }
  }

  async function resumeStrategy(strategyId: string) {
    error.resume = null
    try {
      await apiResumeStrategy(strategyId)
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      error.resume = '恢复策略失败'
      throw err
    }
  }

  async function deployStrategy(strategyId: string) {
    error.deploy = null
    try {
      await apiDeployStrategy(strategyId)
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      error.deploy = '部署策略失败'
      throw err
    }
  }

  async function archiveStrategy(strategyId: string) {
    error.archive = null
    try {
      await apiArchiveStrategy(strategyId)
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      error.archive = '归档策略失败'
      throw err
    }
  }

  async function toggleStrategy(strategyId: string, enabled: boolean) {
    error.toggle = null
    try {
      await apiToggleStrategy(strategyId, enabled)
      const found = strategyStore.strategies.find((s) => s.strategy_id === strategyId)
      if (found) {
        found.enabled = enabled
      }
    } catch (err) {
      error.toggle = '更新策略状态失败'
      throw err
    }
  }

  return {
    error,
    startStrategy,
    stopStrategy,
    pauseStrategy,
    resumeStrategy,
    deployStrategy,
    archiveStrategy,
    toggleStrategy,
  }
})
