<template>
  <div class="monitor-dashboard">
    <el-row :gutter="20" class="header">
      <el-col :span="18">
        <h2>实时监控</h2>
      </el-col>
      <el-col :span="6" class="controls">
        <el-button type="primary" @click="refreshData" :loading="loading">刷新数据</el-button>
        <el-button @click="toggleAutoRefresh">
          {{ autoRefresh ? '停止自动刷新' : '开始自动刷新' }}
        </el-button>
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
              <el-table :data="alerts" style="width: 100%">
                <el-table-column prop="timestamp" label="时间" width="180">
                  <template #default="scope">
                    {{ formatDate(scope.row.timestamp) }}
                  </template>
                </el-table-column>
                <el-table-column prop="source" label="来源" width="150" />
                <el-table-column prop="level" label="级别" width="100">
                  <template #default="scope">
                    <el-tag :type="getAlertLevelType(scope.row.level)">
                      {{ getAlertLevelText(scope.row.level) }}
                    </el-tag>
                  </template>
                </el-table-column>
                <el-table-column prop="message" label="消息" />
                <el-table-column label="操作" width="150">
                  <template #default="scope">
                    <el-button 
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
import { ref, onMounted, onUnmounted, watch } from 'vue';
import * as echarts from 'echarts';
import { invoke } from '@tauri-apps/api/core';
import { 
  TrendCharts, 
  Check, 
  Close, 
  Wallet, 
  Coin, 
  Trophy 
} from '@element-plus/icons-vue';

// Reactive data
const activeTab = ref('metrics');
const loading = ref(false);
const autoRefresh = ref(true);
const refreshInterval = ref<number | null>(null);

// Metrics data
const metrics = ref<Record<string, number>>({});
const metricsHistory = ref<Array<{time: string, metrics: Record<string, number>}>>([]);

// Alerts data
const alerts = ref<any[]>([]);

// Logs data
const logs = ref<any[]>([]);
const logLevel = ref('');

// Format numbers
function formatNumber(value: number): string {
  return value.toLocaleString('zh-CN');
}

// Format currency
function formatCurrency(value: number): string {
  return value.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

// Format date
function formatDate(dateInput: string | { timestamp: string }): string {
  let dateString: string;
  
  if (typeof dateInput === 'string') {
    dateString = dateInput;
  } else {
    dateString = dateInput.timestamp;
  }
  
  // Handle different timestamp formats
  if (dateString.endsWith('Z')) {
    // ISO format with Z suffix
    return new Date(dateString).toLocaleString('zh-CN');
  } else if (dateString.includes('.')) {
    // Format with microseconds
    const [mainPart, _] = dateString.split('.');
    return new Date(mainPart + 'Z').toLocaleString('zh-CN');
  } else {
    return new Date(dateString).toLocaleString('zh-CN');
  }
}

// Get alert level type for Element Plus tag
function getAlertLevelType(level: string): 'success' | 'warning' | 'danger' | 'info' | '' {
  switch (level) {
    case 'Info': return 'info';
    case 'Warning': return 'warning';
    case 'Critical': return 'danger';
    default: return 'info';
  }
}

// Get alert level text
function getAlertLevelText(level: string): string {
  switch (level) {
    case 'Info': return '信息';
    case 'Warning': return '警告';
    case 'Critical': return '严重';
    default: return level;
  }
}

// Fetch metrics
async function fetchMetrics() {
  try {
    const data = await invoke<Record<string, number>>('get_metrics');
    metrics.value = data;
    console.log('Metrics:', data);
    // Add to history for chart
    const now = new Date().toLocaleTimeString('zh-CN');
    metricsHistory.value.push({
      time: now,
      metrics: { ...data }
    });
    
    // Keep only last 20 data points
    if (metricsHistory.value.length > 20) {
      metricsHistory.value.shift();
    }
    
    // Update chart
    updateMetricsChart();
  } catch (error) {
    console.error('Failed to fetch metrics:', error);
    // Mock data for web development
    metrics.value = {
      orders_total: Math.floor(Math.random() * 1000),
      orders_filled: Math.floor(Math.random() * 800),
      orders_cancelled: Math.floor(Math.random() * 200),
      account_balance: 1234567.89 + Math.random() * 10000,
      position_value: 1000000 + Math.random() * 50000,
      daily_pnl: 12345.67 + Math.random() * 1000
    };
  }
}

// Fetch alerts
async function fetchAlerts() {
  try {
    const data = await invoke<any[]>('get_alerts');
    alerts.value = data;
  } catch (error) {
    console.error('Failed to fetch alerts:', error);
    // Mock data for web development
    alerts.value = [
      {
        alert_id: '1',
        level: 'Warning',
        source: 'Risk Management',
        message: 'Account margin ratio approaching limit',
        timestamp: new Date().toISOString(),
        acknowledged: false
      },
      {
        alert_id: '2',
        level: 'Critical',
        source: 'Trading Engine',
        message: 'Order execution latency exceeded threshold',
        timestamp: new Date(Date.now() - 300000).toISOString(),
        acknowledged: false
      }
    ];
  }
}

// Acknowledge alert
async function acknowledgeAlert(alertId: string) {
  try {
    await invoke<boolean>('acknowledge_alert', { alertId });
    
    // Update local state
    const alert = alerts.value.find(a => a.alert_id === alertId);
    if (alert) {
      alert.acknowledged = true;
    }
  } catch (error) {
    console.error('Failed to acknowledge alert:', error);
    // Mock for web development
    const alert = alerts.value.find(a => a.alert_id === alertId);
    if (alert) {
      alert.acknowledged = true;
    }
    console.log('Acknowledging alert:', alertId);
  }
}

// Fetch logs
async function fetchLogs() {
  try {
    const data = await invoke<any[]>('get_logs', { 
      level: logLevel.value || null, 
      limit: 50 
    });
    logs.value = data;
    console.log('Logs:', data);
  } catch (error) {
    console.error('Failed to fetch logs:', error);
    // Mock logs for now
    logs.value = [
      {
        timestamp: new Date().toISOString(),
        level: 'info',
        message: 'System started successfully111'
      },
      {
        timestamp: new Date(Date.now() - 60000).toISOString(),
        level: 'warning',
        message: 'Account margin ratio approaching limit'
      },
      {
        timestamp: new Date(Date.now() - 120000).toISOString(),
        level: 'error',
        message: 'Order execution failed for symbol 600519.SH'
      }
    ];
  }
}

// Refresh all data
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

// Toggle auto refresh
function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value;
  if (autoRefresh.value) {
    startAutoRefresh();
  } else {
    stopAutoRefresh();
  }
}

// Start auto refresh
function startAutoRefresh() {
  if (refreshInterval.value) {
    clearInterval(refreshInterval.value);
  }
  refreshInterval.value = window.setInterval(() => {
    refreshData();
  }, 5000); // Refresh every 5 seconds
}

// Stop auto refresh
function stopAutoRefresh() {
  if (refreshInterval.value) {
    clearInterval(refreshInterval.value);
    refreshInterval.value = null;
  }
}

// Initialize metrics chart
function initMetricsChart() {
  const chartDom = document.getElementById('metrics-chart');
  if (chartDom) {
    const chart = echarts.init(chartDom);
    
    const option = {
      tooltip: {
        trigger: 'axis'
      },
      legend: {
        data: ['总订单数', '已成交订单', '账户余额', '今日盈亏']
      },
      xAxis: {
        type: 'category',
        data: []
      },
      yAxis: {
        type: 'value'
      },
      series: [
        {
          name: '总订单数',
          type: 'line',
          data: [],
          smooth: true
        },
        {
          name: '已成交订单',
          type: 'line',
          data: [],
          smooth: true
        },
        {
          name: '账户余额',
          type: 'line',
          data: [],
          smooth: true
        },
        {
          name: '今日盈亏',
          type: 'line',
          data: [],
          smooth: true
        }
      ]
    };
    
    chart.setOption(option);
  }
}

// Update metrics chart
function updateMetricsChart() {
  const chartDom = document.getElementById('metrics-chart');
  if (chartDom && metricsHistory.value.length > 0) {
    const chart = echarts.getInstanceByDom(chartDom) || echarts.init(chartDom);
    
    const times = metricsHistory.value.map(item => item.time);
    const ordersTotal = metricsHistory.value.map(item => item.metrics.orders_total || 0);
    const ordersFilled = metricsHistory.value.map(item => item.metrics.orders_filled || 0);
    const accountBalance = metricsHistory.value.map(item => item.metrics.account_balance || 0);
    const dailyPnl = metricsHistory.value.map(item => item.metrics.daily_pnl || 0);
    
    chart.setOption({
      xAxis: {
        data: times
      },
      series: [
        {
          name: '总订单数',
          data: ordersTotal
        },
        {
          name: '已成交订单',
          data: ordersFilled
        },
        {
          name: '账户余额',
          data: accountBalance
        },
        {
          name: '今日盈亏',
          data: dailyPnl
        }
      ]
    });
  }
}

// Initialize on mount
onMounted(async () => {
  initMetricsChart();
  await refreshData();
  startAutoRefresh();
});

// Clean up on unmount
onUnmounted(() => {
  stopAutoRefresh();
});

// Watch for tab changes
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