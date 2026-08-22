<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted, watch, nextTick } from 'vue'
// NOTE: chartInstance uses shallowRef (no deep proxy needed for ECharts instance)
import * as echarts from 'echarts'
import { listen, type UnlistenFn, type Event } from '@tauri-apps/api/event'
import type { WsCandle } from '@/services/types'

const props = defineProps<{
  symbol: string
}>()

const chartRef = ref<HTMLDivElement>()
const chartInstance = shallowRef<echarts.ECharts | null>(null)
const candleData = ref<WsCandle[]>([])
const MAX_CANDLES = 500

const period = ref<'1m' | '5m' | '15m' | '1H'>('1m')
const periods: { label: string; value: '1m' | '5m' | '15m' | '1H' }[] = [
  { label: '1m', value: '1m' },
  { label: '5m', value: '5m' },
  { label: '15m', value: '15m' },
  { label: '1H', value: '1H' },
]

let unlistenCandle: UnlistenFn | null = null

function formatTime(ts: string): string {
  const d = new Date(Number(ts))
  const h = String(d.getHours()).padStart(2, '0')
  const m = String(d.getMinutes()).padStart(2, '0')
  const s = String(d.getSeconds()).padStart(2, '0')
  return `${h}:${m}:${s}`
}

function buildBaseOption(): echarts.EChartsOption {
  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'cross' },
      formatter: (params: unknown) => {
        const pArr = Array.isArray(params) ? params : [params]
        const p = pArr[0]
        if (
          p === null ||
          p === undefined ||
          typeof p !== 'object' ||
          !('data' in p)
        ) {
          return ''
        }
        const v = (p as { data?: number[] }).data
        if (!v || v.length < 4) return ''
        const name = (p as { name?: string }).name ?? ''
        return [
          `Time: ${name}`,
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
        data: [],
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

function initChart() {
  if (!chartRef.value) return
  if (chartInstance.value) {
    chartInstance.value.dispose()
    chartInstance.value = null
  }
  chartInstance.value = echarts.init(chartRef.value)
  chartInstance.value.setOption(buildBaseOption())
}

function updateIncremental(data: WsCandle[]) {
  if (!chartInstance.value || data.length === 0) return

  const times = data.map((d) => formatTime(d.ts))
  const values = data.map((d) => [
    parseFloat(d.o),
    parseFloat(d.c),
    parseFloat(d.l),
    parseFloat(d.h),
  ])

  chartInstance.value.setOption(
    {
      xAxis: { data: times },
      series: [
        {
          type: 'candlestick',
          data: values,
        },
      ],
    },
    { notMerge: false }
  )
}

function onCandleEvent(event: Event<WsCandle>) {
  const payload = event.payload
  if (payload.inst_id !== props.symbol) return

  const existingIndex = candleData.value.findIndex((d) => d.ts === payload.ts)
  if (existingIndex >= 0) {
    candleData.value[existingIndex] = payload
  } else {
    candleData.value.push(payload)
    if (candleData.value.length > MAX_CANDLES) {
      candleData.value.shift()
    }
  }
  updateIncremental(candleData.value)
}

async function startListening() {
  if (unlistenCandle) {
    unlistenCandle()
    unlistenCandle = null
  }
  unlistenCandle = await listen<WsCandle>('ws:candle', onCandleEvent)
}

function stopListening() {
  if (unlistenCandle) {
    unlistenCandle()
    unlistenCandle = null
  }
}

function resetData() {
  candleData.value = []
  if (chartInstance.value) {
    chartInstance.value.setOption(
      {
        xAxis: { data: [] },
        series: [{ type: 'candlestick', data: [] }],
      },
      { notMerge: false }
    )
  }
}

watch(
  () => props.symbol,
  () => {
    resetData()
  }
)

watch(period, () => {
  resetData()
})

onMounted(() => {
  nextTick(() => {
    initChart()
    startListening()
  })
})

onUnmounted(() => {
  stopListening()
  chartInstance.value?.dispose()
  chartInstance.value = null
})
</script>

<template>
  <div class="chart-card">
    <div class="chart-header">
      <span class="chart-title">{{ symbol }} — Realtime Candle</span>
      <el-radio-group v-model="period" size="small">
        <el-radio-button
          v-for="p in periods"
          :key="p.value"
          :label="p.value"
        >
          {{ p.label }}
        </el-radio-button>
      </el-radio-group>
    </div>
    <div
      v-if="candleData.length === 0"
      class="chart-empty"
    >
      Waiting for data...
    </div>
    <div
      v-show="candleData.length > 0"
      ref="chartRef"
      class="chart-container"
    />
  </div>
</template>

<style scoped>
.chart-card {
  background: #fff;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
  padding: 16px;
}

.chart-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.chart-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.chart-empty {
  height: 400px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-secondary);
  font-size: 14px;
}

.chart-container {
  height: 400px;
}
</style>
