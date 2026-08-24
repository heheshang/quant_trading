<template>
  <div class="monitor-dashboard">
    <MonitorHeader
      :loading="loading"
      :is-polling-fallback="isPollingFallback"
      @refresh="refreshData"
    />
    <el-tabs v-model="activeTab" class="monitor-tabs">
      <el-tab-pane label="指标监控" name="metrics">
        <MetricsCards :metrics="metrics" />
        <MetricsChart
          :metrics-history="metricsHistory"
          :selected-metrics="selectedMetrics"
          @update:selected-metrics="selectedMetrics = $event"
        />
      </el-tab-pane>
      <el-tab-pane label="告警监控" name="alerts">
        <AlertPanel :alerts="alerts" @acknowledge="acknowledgeAlert" @refresh="fetchAlerts" />
      </el-tab-pane>
      <el-tab-pane label="告警阈值" name="thresholds">
        <ThresholdConfig :config="thresholdConfig" @save="saveThresholds" />
      </el-tab-pane>
      <el-tab-pane label="系统日志" name="logs">
        <SystemLogs
          :logs="logs"
          :log-level="logLevel"
          @update:log-level="logLevel = $event"
          @refresh="fetchLogs"
        />
      </el-tab-pane>
      <el-tab-pane label="审计日志" name="audit">
        <AuditLogs />
      </el-tab-pane>
      <el-tab-pane label="实时行情" name="realtime">
        <el-row :gutter="20">
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
        <el-row :gutter="20" style="margin-top: 20px">
          <el-col :xs="24" :lg="12">
            <TradeStream :trades="marketTrades" />
          </el-col>
          <el-col :xs="24" :lg="12">
            <OrderBookDepth :order-book="marketOrderBook" />
          </el-col>
        </el-row>
        <el-row :gutter="20" style="margin-top: 20px">
          <el-col :xs="24">
            <RealtimeCandleChart :candles="marketCandles" :symbol="marketActiveSymbol" />
          </el-col>
        </el-row>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, shallowRef, onMounted, onUnmounted, onActivated, watch } from 'vue'
import { getMetrics, getAlerts, getLogs, acknowledgeAlert as apiAcknowledgeAlert, getThresholds, saveThresholds as persistThresholds } from '@/services/monitor'
import { ElMessage } from 'element-plus'
import type { Alert, LogEntry } from '@/services/types'
import type { ThresholdConfig as ThresholdConfigType } from '@/components/monitor/types'
import MonitorHeader from '@/components/monitor/MonitorHeader.vue'
import MetricsCards from '@/components/monitor/MetricsCards.vue'
import MetricsChart from '@/components/monitor/MetricsChart.vue'
import AlertPanel from '@/components/monitor/AlertPanel.vue'
import ThresholdConfig from '@/components/monitor/ThresholdConfig.vue'
import SystemLogs from '@/components/monitor/SystemLogs.vue'
import AuditLogs from '@/components/monitor/AuditLogs.vue'
import { useMarketDataStore } from '@/stores/marketData'
import RealtimeTickerPanel from '@/components/dashboard/RealtimeTickerPanel.vue'
import TradeStream from '@/components/dashboard/TradeStream.vue'
import OrderBookDepth from '@/components/dashboard/OrderBookDepth.vue'
import RealtimeCandleChart from '@/components/dashboard/RealtimeCandleChart.vue'
import SubscriptionManager from '@/components/dashboard/SubscriptionManager.vue'

const activeTab = ref('metrics')
const loading = ref(false)
const isPollingFallback = ref(true)
const selectedMetrics = ref<string[]>(['orders_total', 'orders_filled', 'orders_cancelled', 'account_balance', 'daily_pnl'])
const metrics = ref<Record<string, number>>({})
const metricsHistory = ref<Array<{ time: string; metrics: Record<string, number> }>>([])
const alerts = shallowRef<Alert[]>([])
const logs = shallowRef<LogEntry[]>([])
const logLevel = ref('')
const thresholdConfig = ref<ThresholdConfigType>({ maxDrawdown: 20, dailyLoss: 10, concentration: 50, leverage: 3, orderLatency: 1000, varWarning: 5 })
thresholdConfig.value = getThresholds()
const marketStore = useMarketDataStore()
const marketTickers = computed(() => marketStore.tickerList)
const marketTrades = computed(() => marketStore.tradesForActive)
const marketOrderBook = computed(() => marketStore.orderBookForActive)
const marketCandles = computed(() => marketStore.candlesForActive)
const marketActiveSymbol = computed(() => marketStore.activeSymbol)
const marketRunning = computed(() => marketStore.running)
const marketStatus = computed(() => marketStore.status)
const marketSymbols = computed(() => marketStore.symbols)

function saveThresholds(cfg: ThresholdConfigType) {
  thresholdConfig.value = cfg
  persistThresholds(cfg)
  ElMessage.success('阈值配置已保存')
}

let pollIntervalId: ReturnType<typeof setInterval> | undefined

async function fetchMetrics() {
  try {
    metrics.value = await getMetrics()
    const now = new Date().toLocaleTimeString('zh-CN')
    metricsHistory.value.push({ time: now, metrics: { ...metrics.value } })
    if (metricsHistory.value.length > 20) metricsHistory.value.shift()
  } catch (error) {
    console.error('Failed to fetch metrics:', error)
    metrics.value = {}
  }
}

async function fetchAlerts() {
  try { alerts.value = await getAlerts() }
  catch (error) { console.error('Failed to fetch alerts:', error); alerts.value = [] }
}

async function fetchLogs() {
  try { logs.value = await getLogs(logLevel.value || undefined, 50) }
  catch (error) { console.error('Failed to fetch logs:', error); logs.value = [] }
}

async function acknowledgeAlert(alertId: number) {
  try {
    await apiAcknowledgeAlert(alertId)
    const alert = alerts.value.find(a => a.alert_id === alertId)
    if (alert) { alert.acknowledged = true; alerts.value = [...alerts.value] }
  } catch (error) { console.error('Failed to acknowledge alert:', error) }
}

async function refreshData() {
  loading.value = true
  try { await Promise.all([fetchMetrics(), fetchAlerts(), fetchLogs()]) }
  catch (error) { console.error('Error refreshing data:', error) }
  finally { loading.value = false }
}

onMounted(async () => {
  await refreshData()
  pollIntervalId = setInterval(() => { refreshData() }, 5000)
  void marketStore.start()
})

// Cache-friendly: refresh latest data when the page is re-activated.
onActivated(() => {
  refreshData()
})

onUnmounted(() => {
  if (pollIntervalId !== undefined) { clearInterval(pollIntervalId); pollIntervalId = undefined }
})

watch(logLevel, () => { fetchLogs() })
</script>

<style scoped>
.monitor-dashboard { padding: 20px; }
.monitor-tabs { margin-top: 20px; }
</style>
