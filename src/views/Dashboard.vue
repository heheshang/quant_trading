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
          <div class="header-controls">
            <el-date-picker
              v-model="dateRange"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              size="small"
              @change="onDateRangeChange"
              style="width: 240px; margin-right: 8px"
            />
            <el-button type="primary" @click="refreshData" :loading="loading">
              刷新数据
            </el-button>
          </div>
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
            @click="router.push('/trading')"
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
            @click="router.push('/monitor')"
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
            @click="router.push('/trading')"
          />
        </el-col>
        <el-col :span="6">
          <StatsCard
            title="风险等级"
            value="中"
            :icon="Warning"
            icon-bg="#f56c6c"
            :loading="loading"
            @click="router.push('/risk')"
          />
        </el-col>
      </el-row>

      <!-- Market data row -->
      <el-row :gutter="20" style="margin-top: 20px">
        <el-col :span="24">
          <el-card>
            <template #header>
              <div class="card-header">
                <span><el-icon><Coin /></el-icon> 市场价格</span>
                <el-button size="small" @click="fetchMarketData" :loading="marketLoading">刷新</el-button>
              </div>
            </template>
            <div v-if="marketError" class="market-data-placeholder">
              <el-icon><Warning /></el-icon>
              <span>{{ marketError }}</span>
            </div>
            <div v-else-if="marketData" class="market-data-grid">
              <div class="market-item">
                <span class="market-label">标的</span>
                <span class="market-value">{{ marketData.symbol }}</span>
              </div>
              <div class="market-item">
                <span class="market-label">最新价</span>
                <span class="market-value">{{ marketData.price }}</span>
              </div>
              <div class="market-item">
                <span class="market-label">涨跌幅</span>
                <span class="market-value" :class="{ positive: (marketData.change || 0) >= 0, negative: (marketData.change || 0) < 0 }">
                  {{ marketData.change_percent ?? '-' }}
                </span>
              </div>
              <div class="market-item">
                <span class="market-label">成交量</span>
                <span class="market-value">{{ marketData.volume ?? '-' }}</span>
              </div>
              <div class="market-item">
                <span class="market-label">最高价</span>
                <span class="market-value">{{ marketData.high ?? '-' }}</span>
              </div>
              <div class="market-item">
                <span class="market-label">最低价</span>
                <span class="market-value">{{ marketData.low ?? '-' }}</span>
              </div>
            </div>
            <div v-else class="market-data-placeholder">
              <el-icon><Coin /></el-icon>
              <span>点击刷新加载行情数据</span>
            </div>
          </el-card>
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
            <el-table v-if="recentTrades.length > 0" :data="recentTrades" style="width: 100%">
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
            <EmptyState v-else title="暂无交易" description="开始交易后最近交易将显示在这里" />
          </el-card>
        </el-col>
      </el-row>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import * as echarts from 'echarts'
import { TrendCharts, Promotion, Tickets, Warning, Coin } from '@element-plus/icons-vue'
import { useAccountStore } from '@/stores/account'
import { useOrderStore } from '@/stores/order'
import { useFormatting } from '@/composables/useFormatting'
import { useMarketData } from '@/composables/useMarketData'
import { getMarketData } from '@/services/api'
import StatsCard from '@/components/StatsCard.vue'
import RealtimeTickerPanel from '@/components/dashboard/RealtimeTickerPanel.vue'
import EmptyState from '@/components/common/EmptyState.vue'

const accountStore = useAccountStore()
const orderStore = useOrderStore()
const router = useRouter()
const { formatCurrency, formatDate, formatOrderSide, formatOrderStatus } = useFormatting()

// Date filter
const dateRange = ref<[Date, Date]>([new Date(Date.now() - 30 * 86400000), new Date()])
function onDateRangeChange() {
  refreshData()
}

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

// Market data
const marketData = ref<any>(null)
const marketError = ref('')
const marketLoading = ref(false)

async function fetchMarketData() {
  if (marketLoading.value) return
  marketLoading.value = true
  marketError.value = ''
  try {
    const data = await getMarketData('default')
    marketData.value = data
  } catch (err: any) {
    marketData.value = null
    marketError.value = err?.message?.includes('Not implemented')
      ? '行情数据功能开发中'
      : '获取行情数据失败: ' + (err?.message || '未知错误')
  } finally {
    marketLoading.value = false
  }
}

const equityChartRef = ref<HTMLDivElement>()
const positionChartRef = ref<HTMLDivElement>()
let equityChart: echarts.ECharts | null = null
let positionChart: echarts.ECharts | null = null

function initCharts() {
  if (!equityChartRef.value || !positionChartRef.value) return
  const positions = accountStore.positions

  // Equity chart — load from account history if available, else show empty
  equityChart = echarts.init(equityChartRef.value)
  const equityHistory = (accountStore.accountInfo as any)?.equity_history as [string, number][] | undefined
  if (equityHistory && equityHistory.length > 0) {
    const dates = equityHistory.map(([d]) =>
      new Date(d).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }),
    )
    const values = equityHistory.map(([, v]) => v)
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
  } else {
    equityChart.setOption({
      graphic: {
        elements: [{
          type: 'text',
          key: 'no-data',
          style: { text: '暂无资产历史', fontSize: 16, textAlign: 'center', fill: '#999' },
          position: ['50%', '50%'],
        }],
      },
    })
  }

  // Position pie chart
  positionChart = echarts.init(positionChartRef.value)
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
        elements: [{
          type: 'text',
          key: 'no-data',
          style: { text: '暂无持仓', fontSize: 16, textAlign: 'center', fill: '#999' },
          position: ['50%', '50%'],
        }],
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

.header-controls {
  display: flex;
  align-items: center;
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
