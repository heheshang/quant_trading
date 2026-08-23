<template>
  <el-row :gutter="20" style="margin-top: 20px;">
    <el-col :span="24">
      <el-card>
        <template #header>
          <div class="card-header">
            <span>实时指标趋势</span>
            <el-select
              :model-value="selectedMetrics"
              @update:model-value="emit('update:selectedMetrics', $event)"
              multiple
              placeholder="选择指标"
              size="small"
              style="width:200px"
            >
              <el-option label="总订单数" value="orders_total" />
              <el-option label="已成交订单" value="orders_filled" />
              <el-option label="已撤单数" value="orders_cancelled" />
              <el-option label="账户余额" value="account_balance" />
              <el-option label="持仓价值" value="position_value" />
              <el-option label="今日盈亏" value="daily_pnl" />
            </el-select>
          </div>
        </template>
        <div ref="chartRef" style="height: 320px;"></div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted, watch } from 'vue'
import * as echarts from 'echarts'
import { useChartTheme, getChartSeriesColors } from '@/composables/useChartTheme'

const props = defineProps<{
  metricsHistory: Array<{ time: string; metrics: Record<string, number> }>
  selectedMetrics: string[]
}>()

const emit = defineEmits<{
  'update:selectedMetrics': [value: string[]]
}>()

const chartRef = ref<HTMLElement | null>(null)
const chart = shallowRef<echarts.ECharts | null>(null)

const metricLabels: Record<string, string> = {
  orders_total: '总订单数',
  orders_filled: '已成交订单',
  orders_cancelled: '已撤单数',
  account_balance: '账户余额',
  position_value: '持仓价值',
  daily_pnl: '今日盈亏',
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
      grid: { left: 50, right: 20, bottom: 30, top: 20 },
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
    grid: { left: 50, right: 20, bottom: 40, top: 20 },
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

function initChart() {
  if (!chartRef.value) return
  chart.value?.dispose()
  chart.value = echarts.init(chartRef.value)
  chart.value.setOption(buildOption(), { notMerge: true })
}

function updateChart() {
  const instance = chart.value
  if (!instance) return
  instance.setOption(buildOption(), { notMerge: true })
}

watch(() => props.selectedMetrics, updateChart, { deep: true })
watch(() => props.metricsHistory, updateChart, { deep: true })

let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  initChart()
  if (chartRef.value) {
    resizeObserver = new ResizeObserver(() => {
      chart.value?.resize()
    })
    resizeObserver.observe(chartRef.value)
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
  chart.value?.dispose()
  chart.value = null
})
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
