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
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted, onActivated, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getMetrics, getAlerts, getLogs, acknowledgeAlert as apiAcknowledgeAlert } from '@/services/monitor'
import { useWebSocketStatus } from '@/composables/useWebSocketStatus'
import { useMarketData } from '@/composables/useMarketData'
import { ElMessage } from 'element-plus'
import type { Alert, AlertLevel, LogEntry } from '@/services/types'
import type { ThresholdConfig as ThresholdConfigType } from '@/components/monitor/types'
import MonitorHeader from '@/components/monitor/MonitorHeader.vue'
import MetricsCards from '@/components/monitor/MetricsCards.vue'
import MetricsChart from '@/components/monitor/MetricsChart.vue'
import AlertPanel from '@/components/monitor/AlertPanel.vue'
import ThresholdConfig from '@/components/monitor/ThresholdConfig.vue'
import SystemLogs from '@/components/monitor/SystemLogs.vue'

interface WsAlertPayload { alert_id: number; level: AlertLevel; source: string; message: string; timestamp: string }
interface WsLogPayload { timestamp: string; level: string; message: string; module: string | null }

const { status: wsStatus, startListening: startWsStatusListening } = useWebSocketStatus()
const { startListening: startMarketListening } = useMarketData()

const activeTab = ref('metrics')
const loading = ref(false)
const isPollingFallback = ref(false)
const selectedMetrics = ref<string[]>(['orders_total', 'orders_filled', 'account_balance', 'daily_pnl'])
const metrics = ref<Record<string, number>>({})
const metricsHistory = ref<Array<{ time: string; metrics: Record<string, number> }>>([])
const alerts = shallowRef<Alert[]>([])
const logs = shallowRef<LogEntry[]>([])
const logLevel = ref('')
const thresholdConfig = ref<ThresholdConfigType>({ maxDrawdown: 20, dailyLoss: 10, concentration: 50, leverage: 3, orderLatency: 1000, varWarning: 5 })

const saved = localStorage.getItem('monitor_thresholds')
if (saved) {
  try { Object.assign(thresholdConfig.value, JSON.parse(saved)) } catch { /* ignore */ }
}

function saveThresholds(cfg: ThresholdConfigType) {
  thresholdConfig.value = cfg
  localStorage.setItem('monitor_thresholds', JSON.stringify(cfg))
  ElMessage.success('阈值配置已保存')
}

let disconnectTimerId: ReturnType<typeof setTimeout> | null = null
let pollTimeoutId: ReturnType<typeof setTimeout> | null = null

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

async function pollData() {
  await Promise.all([fetchMetrics(), fetchAlerts(), fetchLogs()])
}

function schedulePoll() {
  if (!isPollingFallback.value) return
  pollTimeoutId = setTimeout(async () => { await pollData(); schedulePoll() }, 5000)
}

function startPollingFallback() { isPollingFallback.value = true; schedulePoll() }
function stopPollingFallback() {
  isPollingFallback.value = false
  if (pollTimeoutId !== null) { clearTimeout(pollTimeoutId); pollTimeoutId = null }
}

watch(wsStatus, (newStatus) => {
  if (newStatus === 'disconnected') {
    disconnectTimerId = setTimeout(() => { startPollingFallback() }, 60000)
  } else if (newStatus === 'connected') {
    if (disconnectTimerId !== null) { clearTimeout(disconnectTimerId); disconnectTimerId = null }
    stopPollingFallback()
  }
})

const unlisteners: UnlistenFn[] = []

async function startWsListeners() {
  let lastMetricsFetch = 0
  unlisteners.push(await listen<unknown>('ws:ticker', () => {
    const now = Date.now()
    if (now - lastMetricsFetch >= 5000) { lastMetricsFetch = now; fetchMetrics() }
  }))
  unlisteners.push(await listen<WsAlertPayload>('ws:alerts', (event) => {
    const p = event.payload
    alerts.value = [{ alert_id: p.alert_id, level: p.level, source: p.source, message: p.message, timestamp: p.timestamp, acknowledged: false }, ...alerts.value]
  }))
  unlisteners.push(await listen<WsLogPayload>('ws:logs', (event) => {
    const p = event.payload
    logs.value = [{ timestamp: p.timestamp, level: p.level, message: p.message, module: p.module }, ...logs.value]
  }))
}

onMounted(async () => {
  await startWsStatusListening()
  await refreshData()
  await startWsListeners()
  startMarketListening()
})

// Cache-friendly: refresh latest data when the page is re-activated.
onActivated(() => {
  refreshData()
})

onUnmounted(() => {
  for (const unlisten of unlisteners) { unlisten() }
  unlisteners.length = 0
  if (disconnectTimerId !== null) { clearTimeout(disconnectTimerId); disconnectTimerId = null }
  stopPollingFallback()
})

watch(logLevel, () => { fetchLogs() })
</script>

<style scoped>
.monitor-dashboard { padding: 20px; }
.monitor-tabs { margin-top: 20px; }
</style>
