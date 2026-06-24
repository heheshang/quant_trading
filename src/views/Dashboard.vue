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

    <el-row :gutter="20">
      <el-col :span="24">
        <div class="dashboard-header">
          <h2 style="margin: 0">仪表盘</h2>
          <el-button type="primary" @click="refreshData" :loading="loading">
            刷新数据
          </el-button>
        </div>
      </el-col>
    </el-row>

    <el-skeleton v-if="loading && !accountStore.totalAssets" :rows="5" animated />

    <div v-else>
      <!-- Real-time overview -->
      <el-row :gutter="20" class="realtime-overview-row">
        <el-col :span="18">
          <RealtimeTickerPanel :symbols="positionSymbols" />
        </el-col>
        <el-col :span="6">
          <el-card class="pnl-card" shadow="never">
            <template #header>
              <div class="card-header">
                <span class="pnl-title">收益概览</span>
                <span class="realtime-indicator">
                  <span class="realtime-dot" />
                  <span class="realtime-text">实时</span>
                </span>
              </div>
            </template>
            <div class="pnl-content">
              <div class="pnl-item">
                <span class="pnl-label">总盈亏</span>
                <span class="pnl-value" :style="{ color: pnlColor(totalPnl) }">
                  {{ totalPnl >= 0 ? '+' : '' }}{{ formatCurrency(totalPnl) }}
                </span>
              </div>
              <div class="pnl-item">
                <span class="pnl-label">未实现盈亏</span>
                <span class="pnl-value" :style="{ color: pnlColor(unrealizedPnl) }">
                  {{ unrealizedPnl >= 0 ? '+' : '' }}{{ formatCurrency(unrealizedPnl) }}
                </span>
              </div>
            </div>
          </el-card>
        </el-col>
      </el-row>

      <!-- Stat cards row -->
      <el-row :gutter="20">
        <el-col :span="6">
          <StatsCard
            title="总资产"
            :value="accountStore.totalAssets"
            format="currency"
            :icon="TrendCharts"
            icon-bg="#409eff"
            :loading="loading"
          />
        </el-col>
        <el-col :span="6">
          <StatsCard
            title="今日收益"
            :value="Math.abs(accountStore.dailyPnl)"
            format="currency"
            :icon="Promotion"
            icon-bg="#67c23a"
            :trend="accountStore.dailyPnl"
            :loading="loading"
          />
        </el-col>
        <el-col :span="6">
          <StatsCard
            title="活跃订单"
            :value="orderStore.orderCount"
            format="number"
            :icon="Tickets"
            icon-bg="#e6a23c"
            :loading="loading"
          />
        </el-col>
        <el-col :span="6">
          <StatsCard
            title="风险等级"
            value="中"
            :icon="Warning"
            icon-bg="#f56c6c"
            :loading="loading"
          />
        </el-col>
      </el-row>

      <!-- Charts row -->
      <el-row :gutter="20" style="margin-top: 20px">
        <el-col :span="16">
          <el-card>
            <template #header>
              <div class="card-header"><span>资产曲线</span></div>
            </template>
            <div ref="equityChartRef" style="height: 400px"></div>
          </el-card>
        </el-col>
        <el-col :span="8">
          <el-card>
            <template #header>
              <div class="card-header"><span>持仓分布</span></div>
            </template>
            <div ref="positionChartRef" style="height: 400px"></div>
          </el-card>
        </el-col>
      </el-row>

      <!-- Recent trades table -->
      <el-row :gutter="20" style="margin-top: 20px">
        <el-col :span="24">
          <el-card>
            <template #header>
              <div class="card-header"><span>最近交易</span></div>
            </template>
            <el-table :data="recentTrades" style="width: 100%">
              <el-table-column prop="time" label="时间" width="180" />
              <el-table-column prop="symbol" label="标的" width="120" />
              <el-table-column prop="side" label="方向" width="100">
                  <template #default="scope">
                    <el-tag v-if="scope?.row" :type="scope.row.side === '买入' ? 'success' : 'danger'">
                      {{ scope.row.side }}
                    </el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="price" label="价格" />
                <el-table-column prop="quantity" label="数量" />
                <el-table-column prop="status" label="状态">
                  <template #default="scope">
                    <el-tag v-if="scope?.row" :type="scope.row.status === '已成交' ? 'success' : 'info'">
                      {{ scope.row.status }}
                    </el-tag>
                  </template>
                </el-table-column>
            </el-table>
          </el-card>
        </el-col>
      </el-row>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import * as echarts from 'echarts'
import { TrendCharts, Promotion, Tickets, Warning } from '@element-plus/icons-vue'
import { useAccountStore } from '@/stores/account'
import { useOrderStore } from '@/stores/order'
import { useFormatting } from '@/composables/useFormatting'
import { useMarketData } from '@/composables/useMarketData'
import StatsCard from '@/components/StatsCard.vue'
import RealtimeTickerPanel from '@/components/dashboard/RealtimeTickerPanel.vue'

const accountStore = useAccountStore()
const orderStore = useOrderStore()
const { formatCurrency, formatDate, formatOrderSide, formatOrderStatus } = useFormatting()

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

const loading = computed(() => accountStore.loading || orderStore.loading)
const error = computed(() => accountStore.error || orderStore.error)

const { startListening, cleanup } = useMarketData()

const positionSymbols = computed(() => accountStore.positions.map((p) => p.symbol))

const totalPnl = computed(() => accountStore.accountInfo?.total_pnl ?? 0)
const unrealizedPnl = computed(() =>
  accountStore.positions.reduce((sum, p) => sum + (Number(p.unrealized_pnl) || 0), 0),
)

const pnlColor = (value: number) => (value >= 0 ? '#f56c6c' : '#67c23a')

const equityChartRef = ref<HTMLDivElement>()
const positionChartRef = ref<HTMLDivElement>()
let equityChart: echarts.ECharts | null = null
let positionChart: echarts.ECharts | null = null

function initCharts() {
  if (!equityChartRef.value || !positionChartRef.value) return

  // Equity chart
  equityChart = echarts.init(equityChartRef.value)
  const dates: string[] = []
  const values: number[] = []
  const today = new Date()
  for (let i = 29; i >= 0; i--) {
    const date = new Date(today)
    date.setDate(date.getDate() - i)
    dates.push(date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }))
    const baseValue = 1200000
    const fluctuation = Math.sin(i / 5) * 50000 + Math.random() * 20000
    values.push(baseValue + fluctuation)
  }

  equityChart.setOption({
    tooltip: {
      trigger: 'axis',
      formatter: (params: any) =>
        `${params[0].axisValue}<br/>¥${formatCurrency(params[0].value)}`,
    },
    xAxis: { type: 'category', data: dates },
    yAxis: {
      type: 'value',
      axisLabel: { formatter: (v: number) => '¥' + (v / 10000).toFixed(0) + '万' },
    },
    series: [
      {
        data: values,
        type: 'line',
        smooth: true,
        areaStyle: {},
        lineStyle: { width: 3 },
        itemStyle: { color: '#409EFF' },
      },
    ],
  })

  // Position pie chart
  positionChart = echarts.init(positionChartRef.value)
  const positions = accountStore.positions
  if (positions.length > 0) {
    positionChart.setOption({
      tooltip: {
        trigger: 'item',
        formatter: (params: any) =>
          `${params.name}<br/>¥${formatCurrency(params.value)} (${params.percent}%)`,
      },
      series: [
        {
          type: 'pie',
          radius: ['40%', '70%'],
          data: positions.map((pos) => ({
            value: Number(pos.market_value),
            name: pos.symbol,
          })),
          emphasis: {
            itemStyle: { shadowBlur: 10, shadowOffsetX: 0, shadowColor: 'rgba(0, 0, 0, 0.5)' },
          },
        },
      ],
    })
  } else {
    positionChart.setOption({
      graphic: {
        elements: [
          {
            type: 'text',
            key: 'no-data',
            style: { text: '暂无持仓', fontSize: 16, textAlign: 'center', fill: '#999' },
            position: ['50%', '50%'],
          },
        ],
      },
    })
  }
}

async function refreshData() {
  await Promise.all([accountStore.refreshAll(), orderStore.fetchActiveOrders(true)])
  await nextTick()
  initCharts()
}

// Re-init charts when data changes
watch([() => accountStore.accountInfo, () => accountStore.positions, () => orderStore.activeOrders], () => {
  nextTick(() => initCharts())
})

let refreshInterval: ReturnType<typeof setInterval> | undefined

onMounted(async () => {
  // Initial fetches with force=true to always load on mount
  await accountStore.fetchAccountInfo(true)
  await accountStore.fetchPositions(true)
  await orderStore.fetchActiveOrders(true)
  await nextTick()
  initCharts()

  // Start real-time market data listener
  startListening()

  // Auto-refresh every 30 seconds
  refreshInterval = setInterval(() => refreshData(), 30_000)
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
  cleanup()
  equityChart?.dispose()
  positionChart?.dispose()
})
</script>

<style scoped>
.dashboard {
  padding: 0;
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.realtime-overview-row {
  margin-bottom: 20px;
}

.pnl-card :deep(.el-card__header) {
  padding: 12px 16px;
}

.pnl-card :deep(.el-card__body) {
  padding: 16px;
}

.pnl-title {
  font-size: 16px;
  font-weight: 600;
}

.realtime-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
}

.realtime-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #67c23a;
  animation: pulse-dot 2s ease-in-out infinite;
}

.realtime-text {
  font-size: 12px;
  color: #67c23a;
  font-weight: 500;
}

@keyframes pulse-dot {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.pnl-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.pnl-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pnl-label {
  font-size: 14px;
  color: #606266;
}

.pnl-value {
  font-size: 16px;
  font-weight: 600;
}
</style>
