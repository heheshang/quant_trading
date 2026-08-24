import { call } from './transport'
import type { BacktestResult, BacktestResultsPage } from './types'

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
  timeframe: string,
): Promise<BacktestResult> {
  return call<BacktestResult>('run_backtest', {
    strategyId: strategy_id,
    startDate: start_date,
    endDate: end_date,
    initialCapital: initial_capital,
    commissionRate: commission_rate,
    slippage,
    symbols,
    timeframe,
  })
}

export function getBacktestResults(
  limit: number,
  offset: number,
): Promise<BacktestResultsPage> {
  return call<BacktestResultsPage>('get_backtest_results', { limit, offset })
}

export function getBacktestResult(id: number): Promise<BacktestResult> {
  return call<BacktestResult>('get_backtest_result', { id: id.toString() })
}

export function deleteBacktestResult(id: number): Promise<boolean> {
  return call<boolean>('delete_backtest_result', { id: id.toString() })
}
