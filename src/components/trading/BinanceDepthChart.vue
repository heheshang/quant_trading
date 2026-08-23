<template>
  <div class="binance-depth-chart">
    <div class="chart-header">
      <span class="title">深度</span>
      <span class="symbol">{{ symbol }}</span>
    </div>
    <p v-if="!hasData" class="chart-hint">等待深度数据…</p>
    <div ref="chartRef" class="chart-container" v-show="hasData" />
  </div>
</template>

<script setup lang="ts">
import { ref, shallowRef, watch, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import type { BinanceWsDepth } from '@/services/types'
import { useChartTheme, getChartSeriesColors } from '@/composables/useChartTheme'

const props = defineProps<{
  symbol: string
  depth: BinanceWsDepth | null
}>()

const chartRef = ref<HTMLDivElement>()
const chartInstance = shallowRef<echarts.ECharts | null>(null)

const hasData = ref(false)

function processAsks(asks: [number, number][]): [number, number][] {
  const sorted = [...asks].sort((a, b) => a[0] - b[0])
  let cum = 0
  return sorted.map(([price, size]) => {
    cum += size
    return [price, cum]
  })
}

function processBids(bids: [number, number][]): [number, number][] {
  const sorted = [...bids].sort((a, b) => b[0] - a[0])
  let cum = 0
  return sorted.map(([price, size]) => {
    cum += size
    return [price, cum]
  })
}

function buildOption(
  asksData: [number, number][],
  bidsData: [number, number][],
): echarts.EChartsOption {
  const theme = useChartTheme().palette.value
  const chartColors = getChartSeriesColors()
  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'cross',
        crossStyle: { color: theme.axisLabel },
      },
    },
    grid: {
      left: '3%',
      right: '3%',
      bottom: '10%',
      top: '10%',
      containLabel: true,
    },
    xAxis: {
      type: 'value',
      name: 'Price',
      nameLocation: 'middle',
      nameGap: 25,
      axisLine: { lineStyle: { color: theme.axisLabel } },
      axisLabel: { color: theme.axisLabel },
      splitLine: { lineStyle: { color: theme.splitLine } },
    },
    yAxis: {
      type: 'value',
      name: 'Cumulative Quantity',
      nameLocation: 'middle',
      nameGap: 45,
      axisLine: { lineStyle: { color: theme.axisLabel } },
      axisLabel: { color: theme.axisLabel },
      splitLine: { lineStyle: { color: theme.splitLine } },
    },
    series: [
      {
        name: 'Asks',
        type: 'line',
        data: asksData,
        step: 'middle',
        lineStyle: { color: chartColors.red, width: 1 },
        itemStyle: { color: chartColors.red },
        areaStyle: { color: 'rgba(245, 108, 108, 0.15)' },
        symbol: 'none',
      },
      {
        name: 'Bids',
        type: 'line',
        data: bidsData,
        step: 'middle',
        lineStyle: { color: chartColors.green, width: 1 },
        itemStyle: { color: chartColors.green },
        areaStyle: { color: 'rgba(103, 194, 58, 0.15)' },
        symbol: 'none',
      },
    ],
  }
}

function updateChart(): void {
  const instance = chartInstance.value
  const depth = props.depth
  if (!instance || !depth) return
  const asksData = processAsks(depth.asks)
  const bidsData = processBids(depth.bids)
  if (asksData.length === 0 && bidsData.length === 0) return
  instance.setOption(buildOption(asksData, bidsData), { notMerge: true })
  hasData.value = true
}

function initChart(): void {
  if (!chartRef.value) return
  if (chartInstance.value) {
    chartInstance.value.dispose()
    chartInstance.value = null
  }
  chartInstance.value = echarts.init(chartRef.value)
  updateChart()
}

watch(
  () => props.depth,
  () => {
    if (!chartInstance.value) {
      initChart()
    } else {
      updateChart()
    }
  },
  { immediate: true },
)

onMounted(() => {
  initChart()
})

onUnmounted(() => {
  chartInstance.value?.dispose()
  chartInstance.value = null
})
</script>

<style scoped>
.binance-depth-chart {
  width: 100%;
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #ebeef5;
}

.title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.symbol {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.chart-container {
  height: 260px;
  width: 100%;
}

.chart-hint {
  margin: 0;
  padding: 24px 16px;
  color: var(--color-text-secondary);
  text-align: center;
  font-size: 13px;
}
</style>
