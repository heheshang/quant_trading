import { invoke } from '@tauri-apps/api/core'
import type { BacktestResult, BacktestResultSummaryRow } from './types'

/**
 * Backtest service.
 *
 * Run backtests and query/delete persisted backtest results.
 */

export function runBacktest(
  strategy_id: string,
  start_date: string,
  end_date: string,
  initial_capital: number,
  commission_rate: number,
  slippage: number,
  symbols: string[],
): Promise<BacktestResult> {
  return invoke<BacktestResult>('run_backtest', {
    strategyId: strategy_id,
    startDate: start_date,
    endDate: end_date,
    initialCapital: initial_capital,
    commissionRate: commission_rate,
    slippage,
    symbols,
  })
}

export function getBacktestResults(
  limit: number,
  offset: number,
): Promise<BacktestResultSummaryRow[]> {
  return invoke<BacktestResultSummaryRow[]>('get_backtest_results', { limit, offset })
}

export function getBacktestResult(id: number): Promise<BacktestResult> {
  return invoke<BacktestResult>('get_backtest_result', { id: id.toString() })
}

export function deleteBacktestResult(id: number): Promise<boolean> {
  return invoke<boolean>('delete_backtest_result', { id: id.toString() })
}
