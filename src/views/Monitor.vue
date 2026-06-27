<template>
  <div class="monitor-dashboard">
    <el-row :gutter="20" class="header">
      <el-col :span="16">
        <h2>实时监控</h2>
      </el-col>
      <el-col :span="8" class="controls">
        <div class="status-area">
          <ConnectionStatus />
          <el-tag v-if="isPollingFallback" type="warning" size="small" class="polling-badge">
            轮询模式
          </el-tag>
          <el-button type="primary" @click="refreshData" :loading="loading">刷新数据</el-button>
        </div>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTab" class="monitor-tabs">
      <!-- 指标监控 -->
      <el-tab-pane label="指标监控" name="metrics">
        <el-row :gutter="20">
          <el-col :span="8">
            <el-card class="metric-card">
              <div class="metric-header">
                <div class="metric-icon" :style="{ background: '#409EFF' }">
                  <el-icon><TrendCharts /></el-icon>
                </div>
                <div class="metric-info">
                  <div class="metric-label">总订单数</div>
                  <div class="metric-value">{{ formatNumber(metrics.orders_total || 0) }}</div>
                </div>
              </div>
            </el-card>
          </el-col>
          
          <el-col :span="8">
            <el-card class="metric-card">
              <div class="metric-header">
                <div class="metric-icon" :style="{ background: '#67C23A' }">
                  <el-icon><Check /></el-icon>
                </div>
                <div class="metric-info">
                  <div class="metric-label">已成交订单</div>
                  <div class="metric-value">{{ formatNumber(metrics.orders_filled || 0) }}</div>
                </div>
              </div>
            </el-card>
          </el-col>
          
          <el-col :span="8">
            <el-card class="metric-card">
              <div class="metric-header">
                <div class="metric-icon" :style="{ background: '#E6A23C' }">
                  <el-icon><Close /></el-icon>
                </div>
                <div class="metric-info">
                  <div class="metric-label">已撤单数</div>
                  <div class="metric-value">{{ formatNumber(metrics.orders_cancelled || 0) }}</div>
                </div>
              </div>
            </el-card>
          </el-col>
        </el-row>
        
        <el-row :gutter="20" style="margin-top: 20px;">
          <el-col :span="8">
            <el-card class="metric-card">
              <div class="metric-header">
                <div class="metric-icon" :style="{ background: '#F56C6C' }">
                  <el-icon><Wallet /></el-icon>
                </div>
                <div class="metric-info">
                  <div class="metric-label">账户余额</div>
                  <div class="metric-value">¥{{ formatCurrency(metrics.account_balance || 0) }}</div>
                </div>
              </div>
            </el-card>
          </el-col>
          
          <el-col :span="8">
            <el-card class="metric-card">
              <div class="metric-header">
                <div class="metric-icon" :style="{ background: '#909399' }">
                  <el-icon><Coin /></el-icon>
                </div>
                <div class="metric-info">
                  <div class="metric-label">持仓价值</div>
                  <div class="metric-value">¥{{ formatCurrency(metrics.position_value || 0) }}</div>
                </div>
              </div>
            </el-card>
          </el-col>
          
          <el-col :span="8">
            <el-card class="metric-card">
              <div class="metric-header">
                <div class="metric-icon" :style="{ background: '#79BBFF' }">
                  <el-icon><Trophy /></el-icon>
                </div>
                <div class="metric-info">
                  <div class="metric-label">今日盈亏</div>
                  <div class="metric-value" :class="{ 
                    positive: (metrics.daily_pnl || 0) > 0, 
                    negative: (metrics.daily_pnl || 0) < 0 
                  }">
                    {{ (metrics.daily_pnl || 0) > 0 ? '+' : '' }}¥{{ formatCurrency(metrics.daily_pnl || 0) }}
                  </div>
                </div>
              </div>
            </el-card>
          </el-col>
        </el-row>
        
        <!-- 实时指标图表 -->
        <el-row :gutter="20" style="margin-top: 20px;">
          <el-col :span="24">
              <el-card>
              <template #header>
                <div class="card-header">
                  <span>实时指标趋势</span>
                  <el-select v-model="selectedMetrics" multiple placeholder="选择指标" size="small" style="width:200px">
                    <el-option label="总订单数" value="orders_total" />
                    <el-option label="已成交订单" value="orders_filled" />
                    <el-option label="已撤单数" value="orders_cancelled" />
                    <el-option label="账户余额" value="account_balance" />
                    <el-option label="持仓价值" value="position_value" />
                    <el-option label="今日盈亏" value="daily_pnl" />
                  </el-select>
                </div>
              </template>
              <div id="metrics-chart" style="height: 400px;"></div>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>
      
      <!-- 告警监控 -->
      <el-tab-pane label="告警监控" name="alerts">
        <el-row :gutter="20">
          <el-col :span="24">
            <el-card>
              <template #header>
                <div class="card-header">
                  <span>最新告警</span>
                  <el-button type="primary" size="small" @click="fetchAlerts">刷新告警</el-button>
                </div>
              </template>
              <el-table v-if="alerts.length > 0" :data="alerts" style="width: 100%">
                <el-table-column prop="timestamp" label="时间" width="180" />
                <el-table-column prop="source" label="来源" width="150" />
                <el-table-column prop="level" label="级别" width="100" />
                <el-table-column prop="message" label="消息" />
                <el-table-column label="操作" width="150">
                  <template #default="scope">
                    <el-button
                      v-if="scope?.row"
                      size="small" 
                      type="primary" 
                      @click="acknowledgeAlert(scope.row.alert_id)"
                      :disabled="scope.row.acknowledged"
                    >
                      {{ scope.row.acknowledged ? '已确认' : '确认' }}
                    </el-button>
                  </template>
                </el-table-column>
              </el-table>
              <EmptyState v-else title="暂无告警" description="当前没有需要处理的告警" />
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>
      
      <!-- 告警阈值 -->
      <el-tab-pane label="告警阈值" name="thresholds">
        <el-row :gutter="20">
          <el-col :span="24">
            <el-card>
              <template #header>
                <div class="card-header">
                  <span>阈值配置</span>
                  <el-button type="primary" size="small" @click="saveThresholds">保存配置</el-button>
                </div>
              </template>
              <el-form label-width="160px">
                <el-row :gutter="20">
                  <el-col :span="12">
                    <el-form-item label="最大回撤阈值">
                      <el-input-number v-model="thresholdConfig.maxDrawdown" :min="0" :max="100" :precision="1" :step="0.5">
                        <template #suffix>%</template>
                      </el-input-number>
                    </el-form-item>
                    <el-form-item label="日亏损阈值">
                      <el-input-number v-model="thresholdConfig.dailyLoss" :min="0" :max="100" :precision="1" :step="0.5">
                        <template #suffix>%</template>
                      </el-input-number>
                    </el-form-item>
                    <el-form-item label="持仓集中度">
                      <el-input-number v-model="thresholdConfig.concentration" :min="0" :max="100" :precision="1" :step="1">
                        <template #suffix>%</template>
                      </el-input-number>
                    </el-form-item>
                  </el-col>
                  <el-col :span="12">
                    <el-form-item label="杠杆率上限">
                      <el-input-number v-model="thresholdConfig.leverage" :min="1" :max="10" :precision="1" :step="0.5">
                        <template #suffix>x</template>
                      </el-input-number>
                    </el-form-item>
                    <el-form-item label="订单延迟告警">
                      <el-input-number v-model="thresholdConfig.orderLatency" :min="0" :max="10000" :step="10">
                        <template #suffix>ms</template>
                      </el-input-number>
                    </el-form-item>
                    <el-form-item label="VaR 预警阈值">
                      <el-input-number v-model="thresholdConfig.varWarning" :min="0" :max="100" :precision="1" :step="0.5">
                        <template #suffix>%</template>
                      </el-input-number>
                    </el-form-item>
                  </el-col>
                </el-row>
              </el-form>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- 系统日志 -->
      <el-tab-pane label="系统日志" name="logs">
        <el-row :gutter="20">
          <el-col :span="24">
            <el-card>
              <template #header>
                <div class="card-header">
                  <span>系统日志</span>
                  <div>
                    <el-select v-model="logLevel" placeholder="日志级别" size="small" @change="fetchLogs">
                      <el-option label="全部" value="" />
                      <el-option label="信息" value="info" />
                      <el-option label="警告" value="warning" />
                      <el-option label="错误" value="error" />
                    </el-select>
                    <el-button size="small" @click="fetchLogs">刷新日志</el-button>
                  </div>
                </div>
              </template>
              <div class="log-container">
                <div 
                  v-for="(log, index) in logs" 
                  :key="index" 
                  class="log-entry"
                  :class="`log-${log.level}`"
                >
                  <span class="log-time">[{{ formatDate(log.timestamp) }}]</span>
                  <span class="log-level">[{{ log.level.toUpperCase() }}]</span>
                  <span class="log-module" v-if="log.module">[{{ log.module }}]</span>
                  <span class="log-message">{{ log.message }}</span>
                </div>
                <div v-if="logs.length === 0" class="no-logs">
                  暂无日志信息
                </div>
              </div>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted, watch } from 'vue';
import * as echarts from 'echarts';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getMetrics, getAlerts, getLogs, acknowledgeAlert as apiAcknowledgeAlert } from '@/services/api';
import { 
  TrendCharts, 
  Check, 
  Close, 
  Wallet, 
  Coin, 
  Trophy 
} from '@element-plus/icons-vue';
import { useWebSocketStatus } from '@/composables/useWebSocketStatus';
import { useMarketData } from '@/composables/useMarketData';
import ConnectionStatus from '@/components/ws/ConnectionStatus.vue';
import EmptyState from '@/components/common/EmptyState.vue';
import type { Alert, AlertLevel, LogEntry } from '@/services/types';
import { ElMessage } from 'element-plus';

// --- WS event payload types ---
interface WsAlertPayload {
alert_id: number
  level: AlertLevel
  source: string
  message: string
  timestamp: string
}

interface WsLogPayload {
  timestamp: string
  level: string
  message: string
  module: string | null
}

// --- WebSocket status (singleton — calls startListening only once globally) ---
const { status: wsStatus, startListening: startWsStatusListening } = useWebSocketStatus();
const { startListening: startMarketListening } = useMarketData();

// --- Reactive data ---
const activeTab = ref('metrics');
const loading = ref(false);
const isPollingFallback = ref(false);

// Metrics selection
const selectedMetrics = ref<string[]>(['orders_total', 'orders_filled', 'account_balance', 'daily_pnl']);

// Metric labels lookup
const metricLabels: Record<string, string> = {
  orders_total: '总订单数',
  orders_filled: '已成交订单',
  orders_cancelled: '已撤单数',
  account_balance: '账户余额',
  position_value: '持仓价值',
  daily_pnl: '今日盈亏',
};

// Watch selected metrics → update chart
watch(selectedMetrics, () => { updateMetricsChart(); }, { deep: true });

// Metrics data
const metrics = ref<Record<string, number>>({});
const metricsHistory = ref<Array<{time: string, metrics: Record<string, number>}>>([]);

// Alerts data (shallowRef for WS-pushed data efficiency)
const alerts = shallowRef<Alert[]>([]);

// Logs data (shallowRef for WS-pushed data efficiency)
const logs = shallowRef<LogEntry[]>([]);
const logLevel = ref('');

// Threshold config
const thresholdConfig = ref({
  maxDrawdown: 20,
  dailyLoss: 10,
  concentration: 50,
  leverage: 3,
  orderLatency: 1000,
  varWarning: 5,
});

function saveThresholds() {
  const cfg = thresholdConfig.value;
  localStorage.setItem('monitor_thresholds', JSON.stringify(cfg));
  ElMessage.success('阈值配置已保存');
}

// Restore saved thresholds
const saved = localStorage.getItem('monitor_thresholds');
if (saved) {
  try { Object.assign(thresholdConfig.value, JSON.parse(saved)); } catch { /* ignore */ }
}

// --- Polling fallback state ---
let disconnectTimerId: ReturnType<typeof setTimeout> | null = null;
let pollTimeoutId: ReturnType<typeof setTimeout> | null = null;

// --- WS listener cleanup ---
const unlisteners: UnlistenFn[] = [];

// --- Format helpers ---
function formatNumber(value: number): string {
  return value.toLocaleString('zh-CN');
}

function formatCurrency(value: number): string {
  return value.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

function formatDate(dateInput: string | { timestamp: string }): string {
  let dateString: string;
  
  if (typeof dateInput === 'string') {
    dateString = dateInput;
  } else {
    dateString = dateInput.timestamp;
  }
  
  if (dateString.endsWith('Z')) {
    return new Date(dateString).toLocaleString('zh-CN');
  } else if (dateString.includes('.')) {
    const [mainPart, _] = dateString.split('.');
    return new Date(mainPart + 'Z').toLocaleString('zh-CN');
  } else {
    return new Date(dateString).toLocaleString('zh-CN');
  }
}

// --- Data fetch functions (polling fallback) ---
async function fetchMetrics() {
  try {
    metrics.value = await getMetrics();
    const now = new Date().toLocaleTimeString('zh-CN');
    metricsHistory.value.push({
      time: now,
      metrics: { ...metrics.value }
    });
    
    if (metricsHistory.value.length > 20) {
      metricsHistory.value.shift();
    }
    
    updateMetricsChart();
  } catch (error) {
    console.error('Failed to fetch metrics:', error);
    // Show empty state instead of mock data — the error message is visible via ElMessage
    metrics.value = {};
  }
}

async function fetchAlerts() {
  try {
    alerts.value = await getAlerts();
  } catch (error) {
    console.error('Failed to fetch alerts:', error);
    alerts.value = [];
  }
}

async function acknowledgeAlert(alertId: number) {
  try {
    await apiAcknowledgeAlert(alertId);
    
    const current = alerts.value;
    const alert = current.find(a => a.alert_id === alertId);
    if (alert) {
      alert.acknowledged = true;
      alerts.value = [...current];
    }
  } catch (error) {
    console.error('Failed to acknowledge alert:', error);
  }
}

async function fetchLogs() {
  try {
    logs.value = await getLogs(logLevel.value || undefined, 50);
  } catch (error) {
    console.error('Failed to fetch logs:', error);
    logs.value = [];
  }
}

// --- Manual refresh ---
async function refreshData() {
  loading.value = true;
  try {
    await Promise.all([
      fetchMetrics(),
      fetchAlerts(),
      fetchLogs()
    ]);
  } catch (error) {
    console.error('Error refreshing data:', error);
  } finally {
    loading.value = false;
  }
}

// --- Polling fallback (setTimeout chain, NOT setInterval) ---
async function pollData() {
  await Promise.all([
    fetchMetrics(),
    fetchAlerts(),
    fetchLogs()
  ]);
}

function schedulePoll() {
  if (!isPollingFallback.value) return;
  pollTimeoutId = setTimeout(async () => {
    await pollData();
    schedulePoll();
  }, 5000);
}

function startPollingFallback() {
  isPollingFallback.value = true;
  schedulePoll();
}

function stopPollingFallback() {
  isPollingFallback.value = false;
  if (pollTimeoutId !== null) {
    clearTimeout(pollTimeoutId);
    pollTimeoutId = null;
  }
}

// --- Watch WS status for disconnect → polling fallback ---
watch(wsStatus, (newStatus) => {
  if (newStatus === 'disconnected') {
    // Start 60s countdown before falling back to polling
    disconnectTimerId = setTimeout(() => {
      startPollingFallback();
    }, 60000);
  } else if (newStatus === 'connected') {
    // Cancel the disconnect timer
    if (disconnectTimerId !== null) {
      clearTimeout(disconnectTimerId);
      disconnectTimerId = null;
    }
    // Stop polling fallback if active
    stopPollingFallback();
  }
  // 'reconnecting' status: do nothing, keep waiting
});

// --- ECharts ---
const metricsChart = shallowRef<echarts.ECharts | null>(null);

function initMetricsChart() {
  const chartDom = document.getElementById('metrics-chart');
  if (chartDom) {
    metricsChart.value?.dispose();
    metricsChart.value = echarts.init(chartDom);
    
    const series = selectedMetrics.value.map(key => ({
      name: metricLabels[key] || key,
      type: 'line' as const,
      data: [] as number[],
      smooth: true,
    }));
    
    const option = {
      tooltip: { trigger: 'axis' as const },
      legend: { data: series.map(s => s.name) },
      xAxis: { type: 'category' as const, data: [] as string[] },
      yAxis: { type: 'value' as const },
      series,
    };
    
    metricsChart.value.setOption(option);
  }
}

function updateMetricsChart() {
  const chart = metricsChart.value;
  if (chart && metricsHistory.value.length > 0) {
    const times = metricsHistory.value.map(item => item.time);
    const series = selectedMetrics.value.map(key => ({
      name: metricLabels[key] || key,
      data: metricsHistory.value.map(item => item.metrics[key] || 0),
    }));
    
    chart.setOption({
      xAxis: { data: times },
      series,
    });
  }
}

// --- WS event listeners ---
async function startWsListeners() {
  // Listen for ticker events → refresh metrics (throttled to 5s)
  let lastMetricsFetch = 0;
  const tickerUnlisten = await listen<unknown>('ws:ticker', () => {
    const now = Date.now();
    if (now - lastMetricsFetch < 5000) return;
    lastMetricsFetch = now;
    fetchMetrics();
  });
  unlisteners.push(tickerUnlisten);

  // Listen for alert events → push to alerts table
  const alertsUnlisten = await listen<WsAlertPayload>('ws:alerts', (event) => {
    const payload = event.payload;
    const newAlert: Alert = {
      alert_id: payload.alert_id,
      level: payload.level,
      source: payload.source,
      message: payload.message,
      timestamp: payload.timestamp,
      acknowledged: false,
    };
    alerts.value = [newAlert, ...alerts.value];
  });
  unlisteners.push(alertsUnlisten);

  // Listen for log events → push to log stream
  const logsUnlisten = await listen<WsLogPayload>('ws:logs', (event) => {
    const payload = event.payload;
    const newLog: LogEntry = {
      timestamp: payload.timestamp,
      level: payload.level,
      message: payload.message,
      module: payload.module,
    };
    logs.value = [newLog, ...logs.value];
  });
  unlisteners.push(logsUnlisten);
}

// --- Lifecycle ---
onMounted(async () => {
  initMetricsChart();
  // Start WS status listener (idempotent singleton)
  await startWsStatusListening();
  // Fire one manual refresh on mount
  await refreshData();
  // Start WS listeners
  await startWsListeners();
  startMarketListening();
});

onUnmounted(() => {
  // Clean up WS listeners
  for (const unlisten of unlisteners) {
    unlisten();
  }
  unlisteners.length = 0;
  // Clean up timers
  if (disconnectTimerId !== null) {
    clearTimeout(disconnectTimerId);
    disconnectTimerId = null;
  }
  stopPollingFallback();
  // Dispose ECharts metrics chart
  metricsChart.value?.dispose();
  metricsChart.value = null;
});

// Watch for tab changes (keep existing behavior)
watch(activeTab, (newTab) => {
  if (newTab === 'metrics') {
    updateMetricsChart();
  }
});
</script>

<style scoped>
.monitor-dashboard {
  padding: 20px;
}

.header {
  margin-bottom: 20px;
  align-items: center;
}

.controls {
  text-align: right;
}

.status-area {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

.polling-badge {
  flex-shrink: 0;
}

.metric-card {
  margin-bottom: 20px;
}

.metric-header {
  display: flex;
  align-items: center;
  gap: 20px;
}

.metric-icon {
  width: 60px;
  height: 60px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: #fff;
}

.metric-info {
  flex: 1;
}

.metric-label {
  font-size: 14px;
  color: #999;
  margin-bottom: 8px;
}

.metric-value {
  font-size: 24px;
  font-weight: bold;
  color: #333;
}

.metric-value.positive {
  color: #67C23A;
}

.metric-value.negative {
  color: #F56C6C;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.monitor-tabs {
  margin-top: 20px;
}

.log-container {
  max-height: 500px;
  overflow-y: auto;
  font-family: monospace;
}

.log-entry {
  padding: 8px 0;
  border-bottom: 1px solid #eee;
}

.log-time {
  color: #999;
  margin-right: 10px;
}

.log-level {
  margin-right: 10px;
  font-weight: bold;
}

.log-module {
  margin-right: 10px;
  color: #909399;
}

.log-info {
  color: #409EFF;
}

.log-warning {
  color: #E6A23C;
}

.log-error {
  color: #F56C6C;
}

.log-message {
  color: #333;
}

.no-logs {
  text-align: center;
  padding: 20px;
  color: #999;
}
</style>
