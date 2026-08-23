<template>
  <el-card class="risk-chart-card">
    <template #header>
      <div class="card-header">
        <span>风险指标趋势</span>
        <el-select
          :model-value="selectedMetrics"
          @update:model-value="emit('update:selectedMetrics', $event)"
          multiple
          placeholder="选择指标"
          size="small"
          style="width:200px"
        >
          <el-option label="VaR (95%)" value="var_95" />
          <el-option label="VaR (99%)" value="var_99" />
          <el-option label="最大回撤" value="max_drawdown" />
          <el-option label="波动率" value="volatility" />
          <el-option label="夏普比率" value="sharpe_ratio" />
          <el-option label="最大持仓比例" value="max_position_size" />
          <el-option label="单日最大亏损" value="max_daily_loss" />
          <el-option label="持仓集中度" value="max_concentration" />
        </el-select>
      </div>
    </template>
    <div ref="chartRef" style="height: 320px;"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted, watch } from 'vue'
import * as echarts from 'echarts'
import { useChartTheme, getChartSeriesColors } from '@/composables/useChartTheme'

const props = withDefaults(
  defineProps<{
    metricsHistory: Array<{ time: string; metrics: Record<string, number> }>
    selectedMetrics?: string[]
  }>(),
  {
    selectedMetrics: () => ['var_95', 'var_99', 'max_drawdown'],
  },
)

const emit = defineEmits<{
  'update:selectedMetrics': [value: string[]]
}>()

const chartRef = ref<HTMLElement | null>(null)
const riskChart = shallowRef<echarts.ECharts | null>(null)

const metricLabels: Record<string, string> = {
  var_95: 'VaR (95%)',
  var_99: 'VaR (99%)',
  max_drawdown: '最大回撤',
  volatility: '波动率',
  sharpe_ratio: '夏普比率',
  max_position_size: '最大持仓比例',
  max_daily_loss: '单日最大亏损',
  max_concentration: '持仓集中度',
}

const chartColorKeys = ['blue', 'green', 'orange', 'purple', 'teal', 'gray', 'red']

function seriesColor(index: number): string {
  const colors = getChartSeriesColors()
  const key = chartColorKeys[index % chartColorKeys.length]
  return colors[key] || '#409eff'
}

function buildOption(): echarts.EChartsCoreOption {
  const theme = useChartTheme().palette.value
  const hasData = props.metricsHistory.length > 0

  if (!hasData) {
    return {
      title: {
        text: '暂无历史数据',
        left: 'center',
        top: 'center',
        textStyle: { color: theme.text, fontSize: 14, fontWeight: 'normal' },
      },
      tooltip: { trigger: 'axis' as const },
      legend: { show: false },
      grid: { left: 60, right: 20, bottom: 40, top: 20 },
      xAxis: { type: 'category' as const, data: [] as string[] },
      yAxis: { type: 'value' as const },
      series: [],
    }
  }

  const times = props.metricsHistory.map(item => item.time)
  const series = props.selectedMetrics.map((key, index) => ({
    name: metricLabels[key] || key,
    type: 'line' as const,
    smooth: true,
    data: props.metricsHistory.map(item => item.metrics[key] ?? 0),
    itemStyle: { color: seriesColor(index) },
    lineStyle: { color: seriesColor(index) },
  }))

  return {
    tooltip: {
      trigger: 'axis' as const,
      backgroundColor: theme.tooltipBg,
      borderColor: theme.tooltipBorder,
      borderWidth: 1,
      textStyle: { color: theme.tooltipText },
    },
    legend: {
      type: 'scroll' as const,
      data: series.map(s => s.name),
      bottom: 0,
      textStyle: { color: theme.axisLabel },
    },
    grid: { left: 60, right: 20, bottom: 40, top: 20 },
    xAxis: {
      type: 'category' as const,
      data: times,
      axisLabel: { color: theme.axisLabel },
    },
    yAxis: {
      type: 'value' as const,
      axisLabel: { color: theme.axisLabel },
      splitLine: { lineStyle: { color: theme.splitLine } },
    },
    series,
  }
}

function initRiskChart() {
  const dom = chartRef.value
  if (!dom) return
  riskChart.value?.dispose()
  riskChart.value = echarts.init(dom)
  riskChart.value.setOption(buildOption(), { notMerge: true })
}

function updateChart() {
  const instance = riskChart.value
  if (!instance) return
  instance.setOption(buildOption(), { notMerge: true })
}

watch(() => props.selectedMetrics, updateChart, { deep: true })
watch(() => props.metricsHistory, updateChart, { deep: true })

let resizeHandler: (() => void) | null = null

onMounted(() => {
  initRiskChart()
  resizeHandler = () => riskChart.value?.resize()
  window.addEventListener('resize', resizeHandler)
})

onUnmounted(() => {
  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler)
    resizeHandler = null
  }
  riskChart.value?.dispose()
  riskChart.value = null
})
</script>

<style scoped>
.risk-chart-card {
  margin-bottom: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
