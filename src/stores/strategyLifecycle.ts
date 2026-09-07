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
  let generation = 0

  async function startStrategy(strategyId: string) {
    const requestGeneration = generation
    error.start = null
    try {
      await apiStartStrategy(strategyId)
      if (requestGeneration !== generation) return
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      if (requestGeneration !== generation) throw err
      error.start = '启动策略失败'
      throw err
    }
  }

  async function stopStrategy(strategyId: string) {
    const requestGeneration = generation
    error.stop = null
    try {
      await apiStopStrategy(strategyId)
      if (requestGeneration !== generation) return
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      if (requestGeneration !== generation) throw err
      error.stop = '停止策略失败'
      throw err
    }
  }

  async function pauseStrategy(strategyId: string) {
    const requestGeneration = generation
    error.pause = null
    try {
      await apiPauseStrategy(strategyId)
      if (requestGeneration !== generation) return
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      if (requestGeneration !== generation) throw err
      error.pause = '暂停策略失败'
      throw err
    }
  }

  async function resumeStrategy(strategyId: string) {
    const requestGeneration = generation
    error.resume = null
    try {
      await apiResumeStrategy(strategyId)
      if (requestGeneration !== generation) return
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      if (requestGeneration !== generation) throw err
      error.resume = '恢复策略失败'
      throw err
    }
  }

  async function deployStrategy(strategyId: string) {
    const requestGeneration = generation
    error.deploy = null
    try {
      await apiDeployStrategy(strategyId)
      if (requestGeneration !== generation) return
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      if (requestGeneration !== generation) throw err
      error.deploy = '部署策略失败'
      throw err
    }
  }

  async function archiveStrategy(strategyId: string) {
    const requestGeneration = generation
    error.archive = null
    try {
      await apiArchiveStrategy(strategyId)
      if (requestGeneration !== generation) return
      await strategyStore.fetchStrategies(true)
    } catch (err) {
      if (requestGeneration !== generation) throw err
      error.archive = '归档策略失败'
      throw err
    }
  }

  async function toggleStrategy(strategyId: string, enabled: boolean) {
    const requestGeneration = generation
    error.toggle = null
    try {
      await apiToggleStrategy(strategyId, enabled)
      if (requestGeneration !== generation) return
      const found = strategyStore.strategies.find((s) => s.strategy_id === strategyId)
      if (found) {
        found.enabled = enabled
      }
    } catch (err) {
      if (requestGeneration !== generation) throw err
      error.toggle = '更新策略状态失败'
      throw err
    }
  }

  function reset() {
    generation += 1
    Object.assign(error, makeLifecycleError())
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
    reset,
  }
})
