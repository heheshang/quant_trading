<template>
  <el-card class="realtime-candle-chart" shadow="never">
    <template #header>
      <div class="card-header">
        <span class="title"><el-icon><TrendCharts /></el-icon> 实时 K 线</span>
        <span v-if="candles.length" class="symbol">{{ symbol }}</span>
      </div>
    </template>

    <div ref="chartRef" class="chart-container" v-show="candles.length" />
    <EmptyState v-if="!candles.length" title="暂无 K 线" description="等待实时 K 线数据…" />
  </el-card>
</template>

<script setup lang="ts">
import { ref, shallowRef, watch, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import { TrendCharts } from '@element-plus/icons-vue'
import type { BinanceWsKline } from '@/services/types'
import EmptyState from '@/components/common/EmptyState.vue'
import { useChartTheme, getChartSeriesColors } from '@/composables/useChartTheme'

const props = defineProps<{
  candles: BinanceWsKline[]
  symbol: string
}>()

const chartRef = ref<HTMLDivElement>()
const chartInstance = shallowRef<echarts.ECharts | null>(null)

function formatTimestamp(ms: number): string {
  const d = new Date(ms)
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  const hours = String(d.getHours()).padStart(2, '0')
  const minutes = String(d.getMinutes()).padStart(2, '0')
  return `${month}-${day} ${hours}:${minutes}`
}

function buildOption(): echarts.EChartsOption {
  const theme = useChartTheme().palette.value
  const chartColors = getChartSeriesColors()
  const sorted = [...props.candles].sort((a, b) => a.open_time - b.open_time)
  const times = sorted.map((c) => formatTimestamp(c.open_time))
  const values = sorted.map((c) => [c.open, c.close, c.low, c.high])
  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'cross' },
      backgroundColor: theme.tooltipBg,
      borderColor: theme.tooltipBorder,
      textStyle: { color: theme.tooltipText },
    },
    xAxis: {
      type: 'category',
      data: times,
      axisLabel: { rotate: 45, color: theme.axisLabel },
      axisLine: { lineStyle: { color: theme.splitLine } },
      axisTick: { lineStyle: { color: theme.splitLine } },
    },
    yAxis: {
      type: 'value',
      scale: true,
      axisLabel: { color: theme.axisLabel },
      splitLine: { lineStyle: { color: theme.splitLine } },
    },
    series: [
      {
        type: 'candlestick',
        data: values,
        itemStyle: {
          color: chartColors.green,
          color0: chartColors.red,
          borderColor: chartColors.green,
          borderColor0: chartColors.red,
        },
      },
    ],
    grid: {
      left: '5%',
      right: '5%',
      bottom: '8%',
      top: '6%',
    },
  }
}

function renderChart(): void {
  const instance = chartInstance.value
  if (!instance) return
  if (!props.candles.length) return
  instance.setOption(buildOption(), { notMerge: true })
}

function initChart(): void {
  if (!chartRef.value) return
  if (chartInstance.value) {
    chartInstance.value.dispose()
    chartInstance.value = null
  }
  chartInstance.value = echarts.init(chartRef.value)
  renderChart()
}

watch(
  () => props.candles,
  () => {
    if (!chartInstance.value) {
      initChart()
    } else {
      renderChart()
    }
  },
  { immediate: true, deep: false },
)

let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  initChart()
  if (chartRef.value) {
    resizeObserver = new ResizeObserver(() => {
      chartInstance.value?.resize()
    })
    resizeObserver.observe(chartRef.value)
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
  chartInstance.value?.dispose()
  chartInstance.value = null
})
</script>

<style scoped>
.realtime-candle-chart :deep(.el-card__header) {
  padding: 12px 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 15px;
  font-weight: 600;
}

.symbol {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.chart-container {
  height: 300px;
  width: 100%;
}
</style>
