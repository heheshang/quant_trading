/** 一条已成交记录（用于已实现盈亏计算）。 */
export interface FilledFill {
  symbol: string
  side: 'Buy' | 'Sell'
  price: number
  quantity: number
  ts: number
}

/**
 * 计算一组成交的已实现盈亏（平均成本法，按 symbol 维护持仓均价）。
 *
 * 按时间顺序处理：买单累积 `avg cost`，卖单按 `(卖价 − 均价) × 数量` 计入
 * 已实现盈亏，并扣减持仓。超过持仓的卖出按 `avg=0` 保守处理。
 */
export function computeRealizedPnl(fills: FilledFill[]): number {
  const pos = new Map<string, { qty: number; cost: number }>()
  let pnl = 0
  const sorted = [...fills].sort((a, b) => a.ts - b.ts)
  for (const f of sorted) {
    const p = pos.get(f.symbol) ?? { qty: 0, cost: 0 }
    if (f.side === 'Buy') {
      const newQty = p.qty + f.quantity
      pos.set(f.symbol, { qty: newQty, cost: p.cost + f.price * f.quantity })
    } else {
      const avg = p.qty > 0 ? p.cost / p.qty : 0
      pnl += (f.price - avg) * f.quantity
      const newQty = Math.max(0, p.qty - f.quantity)
      pos.set(f.symbol, { qty: newQty, cost: avg * newQty })
    }
  }
  return pnl
}
