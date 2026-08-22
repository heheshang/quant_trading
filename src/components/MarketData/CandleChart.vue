<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick, computed } from 'vue'
import * as echarts from 'echarts'
import { useMarketData } from '@/composables/useMarketData'
import type { WsCandle } from '@/services/types'

const props = defineProps<{
  symbol: string
}>()

const { candleData } = useMarketData()

const chartRef = ref<HTMLDivElement>()
let chart: echarts.ECharts | null = null

const period = ref<'1m' | '5m' | '15m' | '1H'>('1m')
const periods: { label: string; value: '1m' | '5m' | '15m' | '1H' }[] = [
  { label: '1m', value: '1m' },
  { label: '5m', value: '5m' },
  { label: '15m', value: '15m' },
  { label: '1H', value: '1H' },
]

const chartData = ref<WsCandle[]>([])
const MAX_CANDLES = 300

function formatTime(ts: string): string {
  const d = new Date(Number(ts))
  const h = String(d.getHours()).padStart(2, '0')
  const m = String(d.getMinutes()).padStart(2, '0')
  const s = String(d.getSeconds()).padStart(2, '0')
  return `${h}:${m}:${s}`
}

function buildOption(data: WsCandle[]): echarts.EChartsOption {
  const times = data.map((d) => formatTime(d.ts))
  const values = data.map((d) => [
    parseFloat(d.o),
    parseFloat(d.c),
    parseFloat(d.l),
    parseFloat(d.h),
  ])

  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'cross' },
      formatter: (params: any) => {
        const p = Array.isArray(params) ? params[0] : params
        const v = p?.data as number[] | undefined
        if (!v || v.length < 4) return ''
        return [
          `Time: ${p.name}`,
          `Open: ${v[0]}`,
          `Close: ${v[1]}`,
          `Low: ${v[2]}`,
          `High: ${v[3]}`,
        ].join('<br>')
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
      type: 'category',
      data: times,
      axisLine: { lineStyle: { color: 'var(--color-text-secondary)' } },
      axisLabel: { color: 'var(--color-text-regular)' },
    },
    yAxis: {
      type: 'value',
      scale: true,
      axisLine: { lineStyle: { color: 'var(--color-text-secondary)' } },
      axisLabel: { color: 'var(--color-text-regular)' },
      splitLine: { lineStyle: { color: '#eee' } },
    },
    dataZoom: [
      { type: 'inside', start: 50, end: 100 },
      {
        type: 'slider',
        start: 50,
        end: 100,
        bottom: 10,
        height: 20,
        handleSize: '80%',
      },
    ],
    series: [
      {
        type: 'candlestick',
        data: values,
        itemStyle: {
          color: '#f56c6c',
          color0: '#67c23a',
          borderColor: '#f56c6c',
          borderColor0: '#67c23a',
        },
      },
    ],
  }
}

function updateChart() {
  if (!chart) return
  const data = chartData.value
  if (data.length === 0) return
  chart.setOption(buildOption(data), { notMerge: false })
}

function initChart() {
  if (!chartRef.value) return
  if (chart) {
    chart.dispose()
    chart = null
  }
  chart = echarts.init(chartRef.value)
  if (chartData.value.length > 0) {
    chart.setOption(buildOption(chartData.value))
  }
}

const currentCandle = computed(() => candleData.value[props.symbol])

watch(currentCandle, (candle) => {
  if (!candle) return
  const existingIndex = chartData.value.findIndex((d) => d.ts === candle.ts)
  if (existingIndex >= 0) {
    chartData.value[existingIndex] = candle
  } else {
    chartData.value.push(candle)
    if (chartData.value.length > MAX_CANDLES) {
      chartData.value.shift()
    }
  }
  updateChart()
})

watch(() => props.symbol, () => {
  chartData.value = []
  nextTick(() => initChart())
})

watch(period, () => {
  chartData.value = []
  nextTick(() => initChart())
})

onMounted(() => {
  nextTick(() => initChart())
})

onUnmounted(() => {
  chart?.dispose()
  chart = null
})
</script>

<template>
  <div>
    <div style="margin-bottom: 12px; display: flex; gap: 8px; align-items: center;">
      <span style="font-size: 14px; color: var(--color-text-regular);">Period:</span>
      <el-button
        v-for="p in periods"
        :key="p.value"
        size="small"
        :type="period === p.value ? 'primary' : 'default'"
        @click="period = p.value"
      >
        {{ p.label }}
      </el-button>
    </div>
    <div v-if="chartData.length === 0" style="height: 400px; display: flex; align-items: center; justify-content: center; color: var(--color-text-secondary);">
      Waiting for data...
    </div>
    <div v-show="chartData.length > 0" ref="chartRef" style="height: 400px;"></div>
  </div>
</template>
