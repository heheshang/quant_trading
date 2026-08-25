<template>
  <div class="dashboard page-stack">
    <el-alert
      v-if="error"
      :title="error"
      type="error"
      show-icon
      closable
      @close="accountStore.error = null"
      style="margin-bottom: 20px"
    />

    <DashboardHeader
      :date-range="dateRange"
      :loading="loading"
      @update:date-range="onDateRangeChange"
      @refresh="refreshData"
    />

    <el-skeleton v-if="loading && !accountStore.totalAssets" :rows="5" animated />

      <div v-else class="page-stack">
      <el-row :gutter="20" class="grid-stretch">
        <el-col :xs="24">
          <PnlOverview :total-pnl="totalPnl" :unrealized-pnl="unrealizedPnl" />
        </el-col>
      </el-row>

      <el-row :gutter="20" class="grid-stretch">
        <el-col :xs="12" :sm="12" :md="6">
          <StatsCard title="总资产" :value="totalAssets" format="currency" :icon="TrendCharts" icon-bg="var(--color-primary)" :loading="loading" @click="router.push('/trading')" />
        </el-col>
        <el-col :xs="12" :sm="12" :md="6">
          <StatsCard title="今日收益" :value="Math.abs(dailyPnl)" format="currency" :icon="Promotion" icon-bg="var(--color-success)" :trend="dailyPnl" :loading="loading" @click="router.push('/monitor')" />
        </el-col>
        <el-col :xs="12" :sm="12" :md="6">
          <StatsCard title="活跃订单" :value="activeOrderCount" format="number" :icon="Tickets" icon-bg="var(--color-warning)" :loading="loading" @click="router.push('/trading')" />
        </el-col>
        <el-col :xs="12" :sm="12" :md="6">
          <StatsCard title="风险等级" :value="riskLevel" :icon="Warning" icon-bg="var(--color-danger)" :loading="loading || riskLevelLoading" @click="router.push('/risk')" />
        </el-col>
      </el-row>

      <MarketOverview />

      <el-row :gutter="20" class="grid-stretch">
        <el-col :xs="24" :lg="16">
          <EquityChart :equity-history="binanceEquityHistory" />
        </el-col>
        <el-col :xs="24" :lg="8">
          <PositionChart :positions="binanceHoldings" />
        </el-col>
      </el-row>
        <el-row :gutter="20" class="grid-stretch">
        <el-col :xs="24" :md="8">
          <SubscriptionManager
            :running="marketRunning"
            :status="marketStatus"
            :symbols="marketSymbols"
            @start="marketStore.start()"
            @stop="marketStore.stop()"
          />
        </el-col>
        <el-col :xs="24" :md="16">
          <RealtimeTickerPanel :tickers="marketTickers" />
        </el-col>
        </el-row>

        <el-row :gutter="20" class="grid-stretch">
        <el-col :xs="24" :lg="12">
          <TradeStream :trades="marketTrades" />
        </el-col>
        <el-col :xs="24" :lg="12">
          <OrderBookDepth :order-book="marketOrderBook" />
        </el-col>
      </el-row>

        <el-row :gutter="20" class="grid-stretch">
        <el-col :xs="24">
          <RealtimeCandleChart :candles="marketCandles" :symbol="marketActiveSymbol" />
        </el-col>
      </el-row>

      <RecentTrades :trades="recentTrades" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { TrendCharts, Promotion, Tickets, Warning } from '@element-plus/icons-vue'
import { useAccountStore } from '@/stores/account'
import { useBinanceAccountOverview } from '@/composables/useBinanceAccountOverview'
import { useOrderStore } from '@/stores/order'
import { getRiskMetrics } from '@/services/risk'
import { useFormatting } from '@/composables/useFormatting'
import StatsCard from '@/components/StatsCard.vue'
import DashboardHeader from '@/components/dashboard/DashboardHeader.vue'
import PnlOverview from '@/components/dashboard/PnlOverview.vue'
import MarketOverview from '@/components/dashboard/MarketOverview.vue'
import EquityChart from '@/components/dashboard/EquityChart.vue'
import PositionChart from '@/components/dashboard/PositionChart.vue'
import RecentTrades from '@/components/dashboard/RecentTrades.vue'
import { useMarketDataStore } from '@/stores/marketData'
import RealtimeTickerPanel from '@/components/dashboard/RealtimeTickerPanel.vue'
import TradeStream from '@/components/dashboard/TradeStream.vue'
import OrderBookDepth from '@/components/dashboard/OrderBookDepth.vue'
import RealtimeCandleChart from '@/components/dashboard/RealtimeCandleChart.vue'
import SubscriptionManager from '@/components/dashboard/SubscriptionManager.vue'

const accountStore = useAccountStore()
const orderStore = useOrderStore()
const router = useRouter()
const { formatCurrency, formatDate, formatOrderSide, formatOrderStatus } = useFormatting()
const marketStore = useMarketDataStore()
const marketTickers = computed(() => marketStore.tickerList)
const marketTrades = computed(() => marketStore.tradesForActive)
const marketOrderBook = computed(() => marketStore.orderBookForActive)
const marketCandles = computed(() => marketStore.candlesForActive)
const marketActiveSymbol = computed(() => marketStore.activeSymbol)
const marketRunning = computed(() => marketStore.running)
const marketStatus = computed(() => marketStore.status)
const marketSymbols = computed(() => marketStore.symbols)

const dateRange = ref<[Date, Date]>([new Date(Date.now() - 30 * 86400000), new Date()])
function onDateRangeChange(value: [Date, Date]) {
  dateRange.value = value
  refreshData()
}

const loading = computed(() => accountStore.loading || orderStore.loading || binanceOverview.loading.value)
const error = computed(() => accountStore.error || orderStore.error)
// 实盘账户概览（Balances×价格=总资产；live_trades 均价=盈亏）。
const binanceOverview = useBinanceAccountOverview()
const totalAssets = computed(() => binanceOverview.totalAssets.value)
// 今日收益 = 今日最新权益 − 今日起始权益（后台每 60s 快照记录，含已实现+浮动，随资产曲线更新）。
const dailyPnl = computed(() => {
  const rows = binanceOverview.equityHistory.value
  if (rows.length < 2) return 0
  const startOfDay = Date.now() - (Date.now() % 86_400_000)
  const today = rows.filter(([ts]) => new Date(ts).getTime() >= startOfDay)
  if (today.length === 0) return 0
  const latest = Number(today[today.length - 1][1]) || 0
  const before = rows.filter(([ts]) => new Date(ts).getTime() < startOfDay)
  const baseline =
    before.length > 0
      ? Number(before[before.length - 1][1]) || 0
      : Number(today[0][1]) || 0
  return latest - baseline
})
const totalPnl = computed(() => binanceOverview.totalPnl.value)
const unrealizedPnl = computed(() => binanceOverview.unrealizedPnl.value)
// 活跃订单数 = 纸面活跃 + 实盘开放单。
const activeOrderCount = computed(
  () => orderStore.orderCount + binanceOverview.liveOpenCount.value,
)
// 组合对象里的 ref 不会在模板自动解包，这里再包一层。
const binanceHoldings = computed(() => binanceOverview.holdings.value)
const binanceEquityHistory = computed(() => binanceOverview.equityHistory.value)
const riskMetrics = ref<Record<string, number>>({})
const riskLevelLoading = ref(false)

function computeRiskLevel(metrics: Record<string, number>): string {
  if (!metrics || Object.keys(metrics).length === 0) return '暂无'
  const var95 = Number(metrics.var_95 ?? 0)
  const var99 = Number(metrics.var_99 ?? 0)
  const maxDrawdown = Number(metrics.max_drawdown ?? 0)
  const maxConcentration = Number(metrics.max_concentration ?? 0)
  const maxPositionSize = Number(metrics.max_position_size ?? 0)
  let score = 0
  if (var95 > 0.05) score += 1
  if (var99 > 0.10) score += 1
  if (maxDrawdown > 0.20) score += 1
  if (maxConcentration > 0.30) score += 1
  if (maxPositionSize > 0.30) score += 1
  if (score >= 3) return '高'
  if (score >= 1) return '中'
  return '低'
}

const riskLevel = computed(() => computeRiskLevel(riskMetrics.value))

const recentTrades = computed(() =>
  orderStore.recentOrders.map((order) => ({
    time: formatDate(order.created_at),
    symbol: order.symbol,
    side: formatOrderSide(order.side),
    price: order.price ? `¥${formatCurrency(order.price)}` : '-',
    quantity: order.quantity.toString(),
    status: formatOrderStatus(order.status),
  })),
)

async function refreshData() {
  await Promise.all([
    accountStore.refreshAll(),
    orderStore.fetchActiveOrders(true),
    orderStore.fetchRecentOrders(true),
    binanceOverview.refresh(true),
  ])
}
async function fetchRiskLevel() {
  riskLevelLoading.value = true
  try {
    riskMetrics.value = await getRiskMetrics()
  } catch {
    riskMetrics.value = {}
  } finally {
    riskLevelLoading.value = false
  }
}

let refreshInterval: ReturnType<typeof setInterval> | undefined

onMounted(async () => {
  await accountStore.fetchAccountInfo(true)
  await accountStore.fetchPositions(true)
  await orderStore.fetchActiveOrders(true)
  await orderStore.fetchRecentOrders(true)
  await binanceOverview.refresh(true)
  fetchRiskLevel()
  refreshInterval = setInterval(() => refreshData(), 30_000)
  await marketStore.start()
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
})
</script>

<style scoped>
.dashboard {
  padding: 0;
}

</style>
