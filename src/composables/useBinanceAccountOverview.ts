import { ref, computed } from 'vue'
import {
  getBinanceBalance,
  getBinanceTickerPrices,
  getLiveTrades,
  getAccountSnapshots,
  recordAccountSnapshot,
} from '@/services/binance'
import type { BinanceBalance, LiveTrade } from '@/services/types'

const USD_STABLES = ['USDT', 'USDC', 'TUSD', 'BUSD', 'FDUSD', 'DAI']
const KEEPALIVE_MS = 30_000

/**
 * Dashboard 实盘账户概览数据源。
 *
 * 从 Binance 拉取账户余额 + 全市场价格 + 本地 live_trades 成交记录，计算：
 * - 总资产（持仓 × 最新价，USDT 计）
 * - 浮动盈亏（(现价 − 平均成本) × 数量，来自 live_trades FILLED 买单均价）
 * - 今日收益（当日已成交买卖实现盈亏）
 */
export function useBinanceAccountOverview() {
  const balances = ref<BinanceBalance[]>([])
  const prices = ref<Record<string, number>>({})
  const liveTrades = ref<LiveTrade[]>([])
  const loading = ref(false)
  let lastFetched = 0

  function priceOf(asset: string): number {
    if (USD_STABLES.includes(asset)) return 1
    return prices.value[asset + 'USDT'] || 0
  }

  /** 按 order_id 平均买入成本（仅 FILLED 买单，加权）。 */
  function avgCost(asset: string): number {
    const pair = `${asset}-USDT`
    let cost = 0
    let qty = 0
    for (const t of liveTrades.value) {
      if (t.symbol !== pair) continue
      if (t.status !== 'FILLED' || t.side !== 'BUY') continue
      const fq = t.filled_quantity || 0
      cost += (t.price || 0) * fq
      qty += fq
    }
    return qty > 0 ? cost / qty : 0
  }

  const totalAssets = computed(() => {
    let sum = 0
    for (const b of balances.value) {
      const qty = (Number(b.free) || 0) + (Number(b.locked) || 0)
      sum += qty * priceOf(b.asset)
    }
    return sum
  })

  /** 浮动盈亏：现货持仓（有均价的部分）按 (现价 − 均价) × 数量。 */
  const unrealizedPnl = computed(() => {
    const costByAsset = new Map<string, number>()
    for (const b of balances.value) {
      const p = priceOf(b.asset)
      if (p <= 0) continue
      const cost = avgCost(b.asset)
      if (cost <= 0) continue
      const qty = (Number(b.free) || 0) + (Number(b.locked) || 0)
      costByAsset.set(b.asset, (p - cost) * qty)
    }
    let sum = 0
    for (const v of costByAsset.values()) sum += v
    return sum
  })

  /** 今日已实现盈亏：当日 FILLED 卖单 minus 买单（按成交价 × 成交量）。 */
  const dailyPnl = computed(() => {
    const startOfDay = Date.now() - (Date.now() % 86_400_000)
    let pnl = 0
    for (const t of liveTrades.value) {
      if (t.status !== 'FILLED') continue
      if (new Date(t.updated_at).getTime() < startOfDay) continue
      const notional = (t.price || 0) * (t.filled_quantity || 0)
      pnl += t.side === 'SELL' ? notional : -notional
    }
    return pnl
  })

  /** 总盈亏 = 当日已实现 + 浮动。 */
  const totalPnl = computed(() => dailyPnl.value + unrealizedPnl.value)

  /** 持仓分布（余额×价格），供 pie 图；取 Top10 + 其他，避免 500+ 项挤压。 */
  const holdings = computed(() => {
    const all = balances.value
      .map((b) => {
        const qty = (Number(b.free) || 0) + (Number(b.locked) || 0)
        const price = priceOf(b.asset)
        const mv = qty * price
        const cost = avgCost(b.asset)
        return {
          symbol: b.asset,
          quantity: qty,
          available_quantity: qty,
          avg_price: cost || price,
          market_value: mv,
          unrealized_pnl: cost > 0 ? (price - cost) * qty : 0,
          realized_pnl: 0,
          updated_at: new Date().toISOString(),
        }
      })
      .filter((p) => p.market_value > 0)
      .sort((a, b) => b.market_value - a.market_value)
    if (all.length <= 10) return all
    const top = all.slice(0, 10)
    const others = all.slice(10).reduce((s, p) => s + p.market_value, 0)
    return [
      ...top,
      {
        symbol: '其他',
        quantity: 0,
        available_quantity: 0,
        avg_price: 0,
        market_value: others,
        unrealized_pnl: 0,
        realized_pnl: 0,
        updated_at: new Date().toISOString(),
      },
    ]
  })

  /** 开放中的实盘单数（NEW / PARTIALLY_FILLED）。 */
  const liveOpenCount = computed(
    () => liveTrades.value.filter((t) => t.status === 'NEW' || t.status === 'PARTIALLY_FILLED').length,
  )

  const equityHistory = ref<[string, number][]>([])

  async function refresh(force = false) {
    if (!force && Date.now() - lastFetched < KEEPALIVE_MS && balances.value.length > 0) return
    loading.value = true
    try {
      const [b, p, t] = await Promise.all([getBinanceBalance(), getBinanceTickerPrices(), getLiveTrades()])
      balances.value = b
      prices.value = p
      liveTrades.value = t
      lastFetched = Date.now()
      // 记录当前权益快照（资产曲线随时间增长）。
      const eq = Number(totalAssets.value) || 0
      if (eq > 0) {
        try {
          await recordAccountSnapshot(eq)
        } catch {
          // 记录失败忽略
        }
      }
      const rows = await getAccountSnapshots('USDT', 200)
      equityHistory.value = rows
        .map((r) => [String(r.ts), Number(r.eq ?? 0)] as [string, number])
        .sort((a, b) => a[0].localeCompare(b[0]))
    } catch {
      // 限流/失败时保留上次数据（降级）
    } finally {
      loading.value = false
    }
  }

  return {
    balances,
    prices,
    liveTrades,
    loading,
    totalAssets,
    unrealizedPnl,
    dailyPnl,
    totalPnl,
    holdings,
    equityHistory,
    liveOpenCount,
    refresh,
  }
}
