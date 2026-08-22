/**
 * Canonical display-formatting composable.
 *
 * Single source of truth (DRY) for all UI formatting: currency, numbers,
 * percentages, dates, and localized order / strategy labels.
 *
 * Numeric inputs are `number | string | null | undefined` so components can
 * pass raw cell values directly. Blank / non-numeric values render as
 * placeholders (`0.00`, `0`, `-`, `0.00%`) instead of throwing or printing
 * `NaN`.
 */

type NumericLike = number | string | null | undefined

const CURRENCY_LOCALE = 'zh-CN'
const DECIMAL_2 = { minimumFractionDigits: 2, maximumFractionDigits: 2 } as const

/** Coerce to a finite number, or `null` when the input is blank / not numeric. */
function toFiniteNumber(value: NumericLike): number | null {
  if (value === null || value === undefined) return null
  const num = typeof value === 'number' ? value : Number(value)
  return Number.isFinite(num) ? num : null
}

export function useFormatting() {
  function formatCurrency(value: NumericLike): string {
    const num = toFiniteNumber(value)
    if (num === null) return '0.00'
    return num.toLocaleString(CURRENCY_LOCALE, DECIMAL_2)
  }

  function formatNumber(value: NumericLike): string {
    const num = toFiniteNumber(value)
    if (num === null) return '0'
    return num.toLocaleString(CURRENCY_LOCALE)
  }

  function formatDate(input: string | Date | null | undefined): string {
    if (!input) return '-'
    return new Date(input).toLocaleString(CURRENCY_LOCALE)
  }

  function formatPercentage(value: NumericLike): string {
    const num = toFiniteNumber(value)
    if (num === null) return '0.00%'
    return (num * 100).toFixed(2) + '%'
  }

  function formatOrderStatus(status: string): string {
    const map: Record<string, string> = {
      Pending: '待提交',
      Submitted: '已提交',
      PartiallyFilled: '部分成交',
      Filled: '已成交',
      Cancelled: '已撤单',
      Rejected: '已拒绝',
      Expired: '已过期',
    }
    return map[status] ?? status
  }

  function formatStrategyType(type: string): string {
    const map: Record<string, string> = {
      TrendFollowing: '趋势跟踪',
      MeanReversion: '均值回归',
      Arbitrage: '套利',
      MarketMaking: '做市',
      Statistical: '统计套利',
      MachineLearning: '机器学习',
      Custom: '自定义',
    }
    return map[type] ?? type
  }

  function formatOrderSide(side: string): string {
    return side === 'Buy' ? '买入' : side === 'Sell' ? '卖出' : side
  }

  return {
    formatCurrency,
    formatNumber,
    formatDate,
    formatPercentage,
    formatOrderStatus,
    formatStrategyType,
    formatOrderSide,
  }
}
