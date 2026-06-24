export function useFormatting() {
  function formatCurrency(value: number | string): string {
    const num = Number(value)
    if (!num && num !== 0) return '0.00'
    return num.toLocaleString('zh-CN', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  }

  function formatNumber(value: number | string): string {
    const num = Number(value)
    if (!num && num !== 0) return '0'
    return num.toLocaleString('zh-CN')
  }

  function formatDate(dateInput: string | Date): string {
    if (!dateInput) return '-'
    return new Date(dateInput).toLocaleString('zh-CN')
  }

  function formatPercentage(value: number | string): string {
    const num = Number(value)
    if (!num && num !== 0) return '0.00%'
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
