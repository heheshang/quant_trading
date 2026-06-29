export function useFormat() {
  function formatCurrency(value: number | null | undefined): string {
    if (value === null || value === undefined) return '0.00'
    return value.toLocaleString('zh-CN', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  }

  function formatPercentage(value: number | null | undefined): string {
    if (value === null || value === undefined) return '0.00%'
    return (value * 100).toFixed(2) + '%'
  }

  function formatNumber(value: number | null | undefined): string {
    if (value === null || value === undefined) return '-'
    return value.toFixed(2)
  }

  function formatDate(dateStr: string | null | undefined): string {
    if (!dateStr) return '-'
    return new Date(dateStr).toLocaleString('zh-CN')
  }

  return { formatCurrency, formatPercentage, formatNumber, formatDate }
}
