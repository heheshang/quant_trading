<template>
  <el-card>
    <div ref="chartContainer" style="height: 400px; width: 100%"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, onUnmounted, nextTick } from 'vue'
import * as echarts from 'echarts'
import type { BacktestResult } from '@/services/types'
import { useFormatting } from '@/composables/useFormatting'
import { useChartTheme, getChartSeriesColors } from '@/composables/useChartTheme'

const props = defineProps<{
  result: BacktestResult | null
}>()

const { formatCurrency } = useFormatting()

const chartContainer = ref<HTMLDivElement | null>(null)
let chartInstance: echarts.ECharts | null = null
let initTimer: ReturnType<typeof setTimeout> | undefined

function initChart() {
  if (!chartContainer.value || !props.result) return

  if (!chartInstance) {
    chartInstance = echarts.getInstanceByDom(chartContainer.value) || echarts.init(chartContainer.value)
  }

  const theme = useChartTheme().palette.value
  const lineColor = getChartSeriesColors().blue
  const curves = props.result.equity_curve
  const dates = curves.map(([date]) => new Date(date).toLocaleDateString('zh-CN'))
  const values = curves.map(([, value]) => value)

  const option: echarts.EChartsOption = {
    tooltip: {
      trigger: 'axis',
      formatter: (params: unknown) => {
        const items = params as Array<{ axisValue?: string; data?: number }>
        if (!items.length || items[0].data == null) return ''
        return `${items[0].axisValue}<br/>¥${formatCurrency(items[0].data)}`
      },
    },
    xAxis: {
      type: 'category',
      data: dates,
      axisLabel: { color: theme.axisLabel },
      axisLine: { lineStyle: { color: theme.axisLabel } },
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        color: theme.axisLabel,
        formatter: (value: number) => (value / 10000).toFixed(0) + '万',
      },
      splitLine: { lineStyle: { color: theme.splitLine } },
    },
    series: [
      {
        data: values,
        type: 'line',
        smooth: true,
        areaStyle: {},
        lineStyle: { width: 2 },
        itemStyle: { color: lineColor },
      },
    ],
  }

  chartInstance.setOption(option, true)
}

function scheduleInit() {
  if (initTimer) clearTimeout(initTimer)
  initTimer = setTimeout(() => {
    initChart()
  }, 100)
}

function refresh() {
  nextTick(() => scheduleInit())
}

watch(
  () => props.result,
  (newVal) => {
    if (newVal) {
      nextTick(() => scheduleInit())
    }
  },
  { deep: false },
)

onUnmounted(() => {
  chartInstance?.dispose()
  chartInstance = null
  if (initTimer) clearTimeout(initTimer)
})

defineExpose({ refresh })
</script>

<style scoped>
/* Chart container is self-contained; no additional styles needed */
</style>
