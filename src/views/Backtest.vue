<template>
  <div class="backtest-system">
    <el-row :gutter="20" class="header">
      <el-col :span="24"><h2>回测系统</h2></el-col>
    </el-row>
    <BacktestConfig
      ref="configRef"
      :strategies="strategies"
      :running="running"
      :templates="templates"
      @run="handleRun"
      @save-template="handleSaveTemplate"
      @load-template="handleLoadTemplate"
      @reset="handleReset"
    />
    <el-card class="backtest-result-card" v-if="backtestResult">
      <template #header>
        <div class="card-header">
          <span>回测结果</span>
          <el-button @click="exportResult">导出结果</el-button>
        </div>
      </template>
      <el-tabs v-model="activeTab">
        <el-tab-pane label="概览" name="overview">
          <BacktestResults :result="backtestResult" />
        </el-tab-pane>
        <el-tab-pane label="收益曲线" name="equity">
          <BacktestChart ref="chartRef" :result="backtestResult" />
        </el-tab-pane>
        <el-tab-pane label="交易记录" name="trades">
          <BacktestTradeList :records="tradeRecords" />
        </el-tab-pane>
      </el-tabs>
    </el-card>
    <BacktestHistory
      :history-records="historyRecords"
      :history-loading="historyLoading"
      @view-detail="viewHistoryDetail"
      @refresh="fetchHistory"
      @delete-record="deleteHistoryRecord"
    />
    <el-card class="loading-card" v-if="running">
      <div class="loading-content">
        <el-skeleton animated>
          <template #template>
            <el-skeleton-item variant="text" style="width: 30%" />
            <el-skeleton-item variant="text" style="width: 50%" />
            <el-skeleton-item variant="text" style="width: 70%" />
          </template>
        </el-skeleton>
        <div class="loading-text">正在执行回测，请稍候...</div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { getStrategies } from '@/services/strategy'
import {
  runBacktest as apiRunBacktest,
  getBacktestResults,
  getBacktestResult,
  deleteBacktestResult,
} from '@/services/backtest'
import type { BacktestResult, BacktestResultSummaryRow, StrategyParams } from '@/services/types'
import { ElMessage } from 'element-plus'
import { useFormatting } from '@/composables/useFormatting'
import BacktestConfig from '@/components/backtest/BacktestConfig.vue'
import type { BacktestConfigData, ConfigTemplate } from '@/components/backtest/BacktestConfig.vue'
import BacktestResults from '@/components/backtest/BacktestResults.vue'
import BacktestChart from '@/components/backtest/BacktestChart.vue'
import BacktestTradeList from '@/components/backtest/BacktestTradeList.vue'
import type { TradeRecord } from '@/components/backtest/BacktestTradeList.vue'
import BacktestHistory from '@/components/backtest/BacktestHistory.vue'
type BacktestResultWithTrades = BacktestResult & { trades?: TradeRecord[] }
const strategies = ref<StrategyParams[]>([])
const backtestResult = ref<BacktestResult | null>(null)
const tradeRecords = ref<TradeRecord[]>([])
const activeTab = ref('overview')
const running = ref(false)
const templates = ref<ConfigTemplate[]>([])
const historyRecords = ref<BacktestResultSummaryRow[]>([])
const historyLoading = ref(false)
const configRef = ref<InstanceType<typeof BacktestConfig>>()
const chartRef = ref<InstanceType<typeof BacktestChart>>()
const { formatCurrency, formatPercentage, formatNumber } = useFormatting()
// ------ Template management ------
function handleSaveTemplate(config: BacktestConfigData) {
  const name = prompt('输入模板名称：')
  if (!name) return
  templates.value.push({ name, config })
  ElMessage.success(`模板「${name}」已保存`)
}

function handleLoadTemplate(index: number) {
  const tpl = templates.value[index]
  if (tpl) {
    configRef.value?.setConfig(tpl.config)
    ElMessage.success(`已加载模板「${tpl.name}」`)
  }
}

function handleReset() {
  backtestResult.value = null
  tradeRecords.value = []
}

// ------ API calls ------
async function fetchStrategies() {
  try {
    strategies.value = await getStrategies()
  } catch {
    ElMessage.error('获取策略列表失败')
  }
}

async function fetchHistory() {
  historyLoading.value = true
  try {
    historyRecords.value = await getBacktestResults(50, 0)
  } catch {
    // Silently fail
  } finally {
    historyLoading.value = false
  }
}

async function viewHistoryDetail(id: number) {
  try {
    backtestResult.value = await getBacktestResult(id)
    activeTab.value = 'overview'
  } catch {
    ElMessage.error('获取回测详情失败')
  }
}

async function deleteHistoryRecord(id: number) {
  try {
    await deleteBacktestResult(id)
    ElMessage.success('删除成功')
    fetchHistory()
  } catch {
    ElMessage.error('删除失败')
  }
}

async function handleRun(config: BacktestConfigData) {
  running.value = true
  try {
    const symbols = config.symbols
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0)
    const result: BacktestResultWithTrades = await apiRunBacktest(
      config.strategyId,
      config.startDate,
      config.endDate,
      config.initialCapital,
      config.commissionRate,
      config.slippage,
      symbols,
    )
    backtestResult.value = result
    tradeRecords.value = result.trades || []
    ElMessage.success('回测完成')
    fetchHistory()
  } catch (error: unknown) {
    ElMessage.error('回测失败: ' + (error as Error).message)
  } finally {
    running.value = false
  }
}

function exportResult() {
  const r = backtestResult.value
  if (!r) return
  const headers = ['指标', '值']
  const rows = [
    ['总收益率', formatPercentage(r.total_return)],
    ['年化收益率', formatPercentage(r.annual_return)],
    ['夏普比率', formatNumber(r.sharpe_ratio)],
    ['最大回撤', formatPercentage(r.max_drawdown)],
    ['胜率', formatPercentage(r.win_rate)],
    ['总交易数', String(r.total_trades)],
    ['初始资金', formatCurrency(r.initial_capital)],
    ['最终资金', formatCurrency(r.final_capital)],
  ]
  const csv = [headers.join(','), ...rows.map((row) => row.join(','))].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `backtest_result_${new Date().toISOString().slice(0, 10)}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

// ------ Lifecycle ------
onMounted(() => {
  fetchStrategies()
  fetchHistory()
})

// ------ Chart refresh when switching to equity tab ------
watch(activeTab, (tab) => {
  if (tab === 'equity' && backtestResult.value) {
    chartRef.value?.refresh()
  }
})

defineExpose({
  strategies,
  backtestResult,
  tradeRecords,
  activeTab,
  running,
  templates,
  historyRecords,
  historyLoading,
  configRef,
  chartRef,
  handleSaveTemplate,
  handleLoadTemplate,
  handleReset,
  fetchStrategies,
  fetchHistory,
  viewHistoryDetail,
  deleteHistoryRecord,
  handleRun,
  exportResult,
})
</script>

<style scoped>
.backtest-system {
  padding: 20px;
}

.header {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.backtest-result-card {
  margin-bottom: 20px;
}

.loading-card {
  text-align: center;
}

.loading-content {
  padding: 40px 20px;
}

.loading-text {
  margin-top: 20px;
  color: var(--color-text-secondary);
}
</style>
