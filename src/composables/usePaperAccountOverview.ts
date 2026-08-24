import { ref, computed } from 'vue'
import { getBinanceTickerPrices } from '@/services/binance'
import { getRecentOrders } from '@/services/order'
import { computeRealizedPnl, type FilledFill } from '@/utils/pnl'
import type { Order } from '@/services/types'

const USD_STABLES = ['USDT', 'USDC', 'TUSD', 'BUSD', 'FDUSD', 'DAI']
const KEEPALIVE_MS = 30_000

/**
 * 纸面账户动态统计。
 *
 * 以初始资金 + 已成交订单（按时间）重放：BUY 扣现金加持仓、SELL 加现金减持仓，
 * 再用当前价格算持仓市值，最后得出总资产/可用资金/当日盈亏（均价法）。
 * 使纸面账户卡随成交真实变化（而非固定 demo 值）。
 */
export function usePaperAccountOverview(initialCash: number) {
  const orders = ref<Order[]>([])
  const prices = ref<Record<string, number>>({})
  const loading = ref(false)
  let lastFetched = 0

  function priceOf(sym: string): number {
    // sym 形如 BTC-USDT；稳定币对按 1（此处为报价资产价值）。
    const quote = sym?.split('-')[1] ?? 'USDT'
    if (USD_STABLES.includes(quote)) {
      // 对 USDT 计价标的，直接用其最新价；否则按 1 占位。
      return prices.value[sym.replace(/-/g, '')] || 1
    }
    return 1
  }

  const account = computed(() => {
    let cash = initialCash
    // 每标的持仓数量 + 累计成本（用于均价/浮动盈亏）。
    const pos = new Map<string, { qty: number; cost: number }>()
    const fills: FilledFill[] = []
    const sorted = [...orders.value].sort(
      (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
    )
    for (const o of sorted) {
      if (o.status !== 'Filled') continue
      const qty = o.filled_quantity || 0
      const price = o.price ?? 0
      const notional = price * qty
      const p = pos.get(o.symbol) ?? { qty: 0, cost: 0 }
      if (o.side === 'Buy') {
        cash -= notional
        pos.set(o.symbol, { qty: p.qty + qty, cost: p.cost + notional })
      } else {
        cash += notional
        const avg = p.qty > 0 ? p.cost / p.qty : 0
        const newQty = Math.max(0, p.qty - qty)
        pos.set(o.symbol, { qty: newQty, cost: avg * newQty })
      }
      fills.push({
        symbol: o.symbol,
        side: o.side as 'Buy' | 'Sell',
        price,
        quantity: qty,
        ts: new Date(o.created_at).getTime(),
      })
    }
    let marketValue = 0
    const holdings = [...pos.entries()]
      .filter(([, pc]) => pc.qty > 0)
      .map(([sym, pc]) => {
        const p = priceOf(sym)
        const avg = pc.qty > 0 ? pc.cost / pc.qty : 0
        const mv = pc.qty * p
        return {
          symbol: sym,
          quantity: pc.qty,
          avg_price: avg,
          market_value: mv,
          unrealized_pnl: (p - avg) * pc.qty,
        }
      })
    for (const h of holdings) marketValue += h.market_value

    const startOfDay = Date.now() - (Date.now() % 86_400_000)
    const todayPnl = computeRealizedPnl(fills.filter((f) => f.ts >= startOfDay))

    return {
      cash,
      marketValue,
      totalAssets: cash + marketValue,
      dailyPnl: todayPnl,
      holdings,
    }
  })

  async function refresh(force = false) {
    if (!force && Date.now() - lastFetched < KEEPALIVE_MS && orders.value.length > 0) return
    loading.value = true
    try {
      const [ord, pr] = await Promise.all([getRecentOrders(100, 'paper'), getBinanceTickerPrices()])
      orders.value = Array.isArray(ord) ? ord : []
      prices.value = pr && typeof pr === 'object' ? pr : {}
      lastFetched = Date.now()
    } catch {
      // 失败保留上次数据
    } finally {
      loading.value = false
    }
  }

  return { orders, prices, loading, account, refresh }
}
