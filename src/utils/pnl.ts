/** 一条已成交记录（用于已实现盈亏计算）。 */
export interface FilledFill {
  symbol: string
  side: 'Buy' | 'Sell'
  price: number
  quantity: number
  ts: number
}

/**
 * 计算一组成交的已实现盈亏（**FIFO 严格配对**，按 symbol）。
 *
 * 按时间顺序处理：买单各批次压入队列；卖单从**队首（最早买入）**开始逐批配对，
 * `(卖价 − 该批买入价) × 匹配量` 计入已实现盈亏，直到卖量被消费完。
 * 超过持仓的卖量按 `price × 剩余量` 计入（保守：无成本则全额盈利）。
 */
export function computeRealizedPnl(fills: FilledFill[]): number {
  const lots = new Map<string, { qty: number; price: number }[]>()
  let pnl = 0
  const sorted = [...fills].sort((a, b) => a.ts - b.ts)
  for (const f of sorted) {
    if (f.side === 'Buy') {
      const l = lots.get(f.symbol) ?? []
      l.push({ qty: f.quantity, price: f.price })
      lots.set(f.symbol, l)
    } else {
      const l = lots.get(f.symbol) ?? []
      let remaining = f.quantity
      while (remaining > 0 && l.length > 0) {
        const lot = l[0]
        const matched = Math.min(lot.qty, remaining)
        pnl += (f.price - lot.price) * matched
        lot.qty -= matched
        remaining -= matched
        if (lot.qty <= 0) l.shift()
      }
      // 卖量超过持仓：无成本基线的部分按全额计（保守）。
      if (remaining > 0) {
        pnl += f.price * remaining
      }
      lots.set(f.symbol, l)
    }
  }
  return pnl
}
