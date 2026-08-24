import { useFormatting } from '@/composables/useFormatting'

export function useTradingUtils() {
  const { formatCurrency, formatNumber, formatDate } = useFormatting()

  function formatTimestamp(ts: string): string {
    if (!ts || ts === '0') return '-'
    return new Date(Number(ts)).toLocaleString('zh-CN')
  }

  function getOrderStatusType(status: string): string {
    switch (status) {
      case 'Pending': return ''
      case 'Submitted': return 'primary'
      case 'PartiallyFilled': return 'warning'
      case 'Filled': return 'success'
      case 'Cancelled': return 'danger'
      default: return 'info'
    }
  }

  function getOrderStatusText(status: string): string {
    const map: Record<string, string> = {
      Pending: '待提交',
      Submitted: '已提交',
      PartiallyFilled: '部分成交',
      Filled: '已成交',
      Cancelled: '已撤单',
    }
    return map[status] || status
  }

  /** 订单交易类型中文标签（限价/市价/止损/算法等）。大小写不敏感（兼容 Binance 大写）。 */
  function getOrderTypeText(orderType: string): string {
    const map: Record<string, string> = {
      MARKET: '市价',
      LIMIT: '限价',
      STOPLOSS: '止损',
      STOPLIMIT: '止损限价',
      TWAP: 'TWAP算法',
      VWAP: 'VWAP算法',
      ICEBERG: '冰山算法',
    }
    return map[orderType.toUpperCase()] || orderType
  }

  return { formatCurrency, formatNumber, formatDate, formatTimestamp, getOrderStatusType, getOrderStatusText, getOrderTypeText }
}
