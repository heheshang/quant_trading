import { call } from './transport'
import type { BacktestResult } from './types'

/**
 * Parameter optimization service.
 *
 * Wraps the `optimize_strategy` Tauri command (GridSearch over a parameter
 * grid). `paramGrid` is either an object mapping param name → array of values
 * (Cartesian product) or a pre-expanded array. `metric` is one of
 * `sharpe_ratio` / `annual_return` / `max_drawdown`; `algorithm` defaults to
 * `grid_search`.
 */

export interface ParameterCombo {
  label: string
  params: Record<string, unknown>
  result: BacktestResult | null
}

export interface OptimizationResult {
  total_combinations: number
  combinations_returned: number
  top_n_requested: number
  combinations: ParameterCombo[]
  best: ParameterCombo | null
}

export interface OptimizeParams {
  strategyId: string
  paramGrid: unknown
  metric: string
  algorithm?: string
  topN?: number
  initialCapital?: number
  startDate?: string
  endDate?: string
}

export function optimizeStrategy(params: OptimizeParams): Promise<OptimizationResult> {
  const args: Record<string, unknown> = {
    strategyId: params.strategyId,
    paramGrid: params.paramGrid,
    metric: params.metric,
  }
  if (params.algorithm !== undefined) args.algorithm = params.algorithm
  if (params.topN !== undefined) args.topN = params.topN
  if (params.initialCapital !== undefined) args.initialCapital = params.initialCapital
  if (params.startDate !== undefined) args.startDate = params.startDate
  if (params.endDate !== undefined) args.endDate = params.endDate
  return call<OptimizationResult>('optimize_strategy', args)
}
