import { ref, computed } from 'vue'
import { getPositions } from '@/services/account'
import { getLiveTrades, getAccountSnapshots } from '@/services/binance'
import { computeRealizedPnl, type FilledFill } from '@/utils/pnl'
import type { LiveTrade, Position } from '@/services/types'

const KEEPALIVE_MS = 30_000

/**
 * Dashboard 实盘账户概览数据源（DB-first）。
 *
 * 数据全部来自数据库（remote WS → DB 导入 → 前端读 DB）：
 * - `positions` 表：币安持仓同步（数量/市值/浮动盈亏，每 60s 由快照写入器刷新）。
 * - `live_trades` 表：本地持久化的实盘成交（策略关联 + 均价/已实现盈亏）。
 * - `account_snapshots` 表：总权益曲线（含稳定币现金）。
 */
export function useBinanceAccountOverview() {
  const positions = ref<Position[]>([])
  const liveTrades = ref<LiveTrade[]>([])
  const loading = ref(false)
  const latestEquity = ref(0)
  let lastFetched = 0

  /** 总资产：优先取最近一次账户权益快照（含稳定币）；否则按持仓市值汇总。 */
  const totalAssets = computed(() => {
    if (latestEquity.value > 0) return latestEquity.value
    return positions.value.reduce((s, p) => s + Number(p.market_value || 0), 0)
  })

  /** 浮动盈亏：来自 DB positions 表（同步时按 现价 − 均价 计算）。 */
  const unrealizedPnl = computed(() =>
    positions.value.reduce((s, p) => s + Number(p.unrealized_pnl || 0), 0),
  )

  /** 今日已实现盈亏：当日已成交的 FIFO 已实现盈亏（来自本地 live_trades）。 */
  const dailyPnl = computed(() => {
    const startOfDay = Date.now() - (Date.now() % 86_400_000)
    const fills: FilledFill[] = []
    for (const t of liveTrades.value) {
      if (t.status !== 'FILLED') continue
      if (new Date(t.updated_at).getTime() < startOfDay) continue
      fills.push({
        symbol: t.symbol,
        side: t.side === 'BUY' ? 'Buy' : 'Sell',
        price: t.price,
        quantity: t.filled_quantity,
        ts: new Date(t.updated_at).getTime(),
      })
    }
    return computeRealizedPnl(fills)
  })

  /** 总盈亏 = 当日已实现 + 浮动。 */
  const totalPnl = computed(() => dailyPnl.value + unrealizedPnl.value)

  /** 持仓分布（DB positions），供 pie 图；取 Top10 + 其他，避免 500+ 项挤压。 */
  const holdings = computed(() => {
    const all = positions.value
      .map((p) => ({
        symbol: p.symbol.replace('-USDT', ''),
        quantity: Number(p.quantity || 0),
        available_quantity: Number(p.available_quantity || 0),
        avg_price: Number(p.avg_price || 0),
        market_value: Number(p.market_value || 0),
        unrealized_pnl: Number(p.unrealized_pnl || 0),
        realized_pnl: Number(p.realized_pnl || 0),
        updated_at: p.updated_at,
      }))
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
    () =>
      liveTrades.value.filter(
        (t) => t.status === 'NEW' || t.status === 'PARTIALLY_FILLED',
      ).length,
  )

  const equityHistory = ref<[string, number][]>([])

  async function refresh(force = false) {
    if (!force && Date.now() - lastFetched < KEEPALIVE_MS && positions.value.length > 0) return
    loading.value = true
    try {
      // 全部从 DB 读：positions（币安持仓同步）/ live_trades（本地成交）/ account_snapshots（权益曲线）。
      const [db, trades, snapshots] = await Promise.all([
        getPositions(),
        getLiveTrades(),
        getAccountSnapshots('USDT', 2000),
      ])
      positions.value = db
      liveTrades.value = trades
      latestEquity.value = snapshots.length ? Number(snapshots[0].eq ?? 0) : 0
      equityHistory.value = snapshots
        .map((r) => [String(r.ts), Number(r.eq ?? 0)] as [string, number])
        .sort((a, b) => a[0].localeCompare(b[0]))
      lastFetched = Date.now()
    } catch {
      // 限流/失败时保留上次数据（降级）
    } finally {
      loading.value = false
    }
  }

  return {
    positions,
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
