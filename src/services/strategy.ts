import { call } from './transport'
import type { StrategyParams, StrategyTypeInfo } from './types'

export function getStrategies(): Promise<StrategyParams[]> {
  return call<StrategyParams[]>('get_strategies')
}

export function saveStrategy(strategy: StrategyParams): Promise<string> {
  return call<string>('save_strategy', { strategy })
}

export function deleteStrategy(strategyId: string): Promise<boolean> {
  return call<boolean>('delete_strategy', { strategyId })
}

export function toggleStrategy(strategyId: string, enabled: boolean): Promise<boolean> {
  return call<boolean>('toggle_strategy', { strategyId, enabled })
}

export function deployStrategy(strategyId: string): Promise<string> {
  return call<string>('deploy_strategy', { strategyId })
}

export function startStrategy(strategyId: string): Promise<string> {
  return call<string>('start_strategy', { strategyId })
}

export function stopStrategy(strategyId: string): Promise<string> {
  return call<string>('stop_strategy', { strategyId })
}

export function pauseStrategy(strategyId: string): Promise<string> {
  return call<string>('pause_strategy', { strategyId })
}

export function resumeStrategy(strategyId: string): Promise<string> {
  return call<string>('resume_strategy', { strategyId })
}

export function archiveStrategy(strategyId: string): Promise<string> {
  return call<string>('archive_strategy', { strategyId })
}

export function listStrategyTypes(): Promise<StrategyTypeInfo[]> {
  return call<StrategyTypeInfo[]>('list_strategy_types')
}

export function getStrategyTypeInfo(typeName: string): Promise<StrategyTypeInfo> {
  return call<StrategyTypeInfo>('get_strategy_type_info', { typeName })
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
  return call<string>('create_strategy', {
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
