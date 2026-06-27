import { invoke } from '@tauri-apps/api/core'
import type { StrategyParams } from './types'

export function getStrategies(): Promise<StrategyParams[]> {
  return invoke<StrategyParams[]>('get_strategies')
}

export function getStrategy(strategyId: string): Promise<StrategyParams> {
  return invoke<StrategyParams>('get_strategy', { strategyId })
}

export function saveStrategy(strategy: StrategyParams): Promise<string> {
  return invoke<string>('save_strategy', { strategy })
}

export function deleteStrategy(strategyId: string): Promise<boolean> {
  return invoke<boolean>('delete_strategy', { strategyId })
}

export function toggleStrategy(strategyId: string, enabled: boolean): Promise<boolean> {
  return invoke<boolean>('toggle_strategy', { strategyId, enabled })
}

export function deployStrategy(strategyId: string): Promise<string> {
  return invoke<string>('deploy_strategy', { strategyId })
}

export function startStrategy(strategyId: string): Promise<string> {
  return invoke<string>('start_strategy', { strategyId })
}

export function stopStrategy(strategyId: string): Promise<string> {
  return invoke<string>('stop_strategy', { strategyId })
}

export function pauseStrategy(strategyId: string): Promise<string> {
  return invoke<string>('pause_strategy', { strategyId })
}

export function resumeStrategy(strategyId: string): Promise<string> {
  return invoke<string>('resume_strategy', { strategyId })
}

export function archiveStrategy(strategyId: string): Promise<string> {
  return invoke<string>('archive_strategy', { strategyId })
}
