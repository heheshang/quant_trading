import { invoke } from '@tauri-apps/api/core'
import type { StrategyParams, StrategyTypeInfo } from './types'

export function getStrategies(): Promise<StrategyParams[]> {
  return invoke<StrategyParams[]>('get_strategies')
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

export function listStrategyTypes(): Promise<StrategyTypeInfo[]> {
  return invoke<StrategyTypeInfo[]>('list_strategy_types')
}

export function getStrategyTypeInfo(typeName: string): Promise<StrategyTypeInfo> {
  return invoke<StrategyTypeInfo>('get_strategy_type_info', { typeName })
}

export function createStrategy(
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
): Promise<string> {
  return invoke<string>('create_strategy', {
    typeName,
    strategyName,
    params,
    enabled,
    maxPosition,
    maxDailyLoss,
    instanceLabel: instanceLabel ?? null,
    description: description ?? null,
    tags: tags ?? [],
    symbols: symbols ?? [],
    userId,
  })
}
