<template>
  <el-card>
    <div ref="chartContainer" style="height: 400px; width: 100%"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, computed, nextTick } from 'vue'
import type { EChartsCoreOption } from 'echarts/core'
import { useEcharts } from '@/composables/useEcharts'
import type { BacktestResult } from '@/services/types'
import { useFormatting } from '@/composables/useFormatting'
import { useChartTheme, getChartSeriesColors } from '@/composables/useChartTheme'

const props = defineProps<{
  result: BacktestResult | null
}>()

const { formatCurrency } = useFormatting()

const chartContainer = ref<HTMLDivElement | null>(null)

const chartOptions = computed<EChartsCoreOption>(() => {
  if (!props.result?.equity_curve) return {}

  const theme = useChartTheme().palette.value
  const lineColor = getChartSeriesColors().blue
  const curves = props.result.equity_curve
  const dates = curves.map(([date]) => new Date(date).toLocaleDateString('zh-CN'))
  const values = curves.map(([, value]) => value)

  return {
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
})

const { resize } = useEcharts(chartContainer, chartOptions)

function refresh() {
  nextTick(resize)
}

defineExpose({ refresh })
</script>

<style scoped>
/* Chart container is self-contained; no additional styles needed */
</style>
