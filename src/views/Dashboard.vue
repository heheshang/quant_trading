<template>
  <div class="dashboard">
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

    <div v-else>
      <el-row :gutter="20" class="realtime-overview-row">
        <el-col :span="18">
          <RealtimeTickerPanel :symbols="positionSymbols" />
        </el-col>
        <el-col :span="6">
          <PnlOverview :total-pnl="totalPnl" :unrealized-pnl="unrealizedPnl" />
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="6">
          <StatsCard title="总资产" :value="accountStore.totalAssets" format="currency" :icon="TrendCharts" icon-bg="#409eff" :loading="loading" @click="router.push('/trading')" />
        </el-col>
        <el-col :span="6">
          <StatsCard title="今日收益" :value="Math.abs(accountStore.dailyPnl)" format="currency" :icon="Promotion" icon-bg="#67c23a" :trend="accountStore.dailyPnl" :loading="loading" @click="router.push('/monitor')" />
        </el-col>
        <el-col :span="6">
          <StatsCard title="活跃订单" :value="orderStore.orderCount" format="number" :icon="Tickets" icon-bg="#e6a23c" :loading="loading" @click="router.push('/trading')" />
        </el-col>
        <el-col :span="6">
          <StatsCard title="风险等级" value="中" :icon="Warning" icon-bg="#f56c6c" :loading="loading" @click="router.push('/risk')" />
        </el-col>
      </el-row>

      <MarketOverview />

      <el-row :gutter="20" style="margin-top: 20px">
        <el-col :span="16">
          <EquityChart :equity-history="accountStore.accountInfo?.equity_history" />
        </el-col>
        <el-col :span="8">
          <PositionChart :positions="accountStore.positions" />
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
import { useOrderStore } from '@/stores/order'
import { useFormatting } from '@/composables/useFormatting'
import { useMarketData } from '@/composables/useMarketData'
import StatsCard from '@/components/StatsCard.vue'
import RealtimeTickerPanel from '@/components/dashboard/RealtimeTickerPanel.vue'
import DashboardHeader from '@/components/dashboard/DashboardHeader.vue'
import PnlOverview from '@/components/dashboard/PnlOverview.vue'
import MarketOverview from '@/components/dashboard/MarketOverview.vue'
import EquityChart from '@/components/dashboard/EquityChart.vue'
import PositionChart from '@/components/dashboard/PositionChart.vue'
import RecentTrades from '@/components/dashboard/RecentTrades.vue'

const accountStore = useAccountStore()
const orderStore = useOrderStore()
const router = useRouter()
const { formatCurrency, formatDate, formatOrderSide, formatOrderStatus } = useFormatting()
const { startListening, cleanup } = useMarketData()

const dateRange = ref<[Date, Date]>([new Date(Date.now() - 30 * 86400000), new Date()])
function onDateRangeChange(value: [Date, Date]) {
  dateRange.value = value
  refreshData()
}

const loading = computed(() => accountStore.loading || orderStore.loading)
const error = computed(() => accountStore.error || orderStore.error)
const positionSymbols = computed(() => accountStore.positions.map((p) => p.symbol))
const totalPnl = computed(() => accountStore.accountInfo?.total_pnl ?? 0)
const unrealizedPnl = computed(() =>
  accountStore.positions.reduce((sum, p) => sum + (Number(p.unrealized_pnl) || 0), 0),
)

const recentTrades = computed(() =>
  orderStore.activeOrders.map((order) => ({
    time: formatDate(order.created_at),
    symbol: order.symbol,
    side: formatOrderSide(order.side),
    price: order.price ? `¥${formatCurrency(order.price)}` : '-',
    quantity: order.quantity.toString(),
    status: formatOrderStatus(order.status),
  })),
)

async function refreshData() {
  await Promise.all([accountStore.refreshAll(), orderStore.fetchActiveOrders(true)])
}

let refreshInterval: ReturnType<typeof setInterval> | undefined

onMounted(async () => {
  await accountStore.fetchAccountInfo(true)
  await accountStore.fetchPositions(true)
  await orderStore.fetchActiveOrders(true)
  startListening()
  refreshInterval = setInterval(() => refreshData(), 30_000)
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
  cleanup()
})
</script>

<style scoped>
.dashboard {
  padding: 0;
}

.realtime-overview-row {
  margin-bottom: 20px;
}
</style>
