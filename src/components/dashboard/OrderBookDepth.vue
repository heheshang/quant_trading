<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { WsOrderBook } from '@/services/types'

const props = defineProps<{
  symbol: string
}>()

const chartRef = ref<HTMLDivElement | null>(null)
const chartInstance = shallowRef<echarts.ECharts | null>(null)
const rawOrderbook = shallowRef<WsOrderBook | null>(null)
const lastUpdateTime = ref<number>(0)

let unlistenFn: UnlistenFn | null = null

interface TooltipParam {
  marker?: string
  seriesName?: string
  data?: [number, number]
}

function isTooltipParam(p: unknown): p is TooltipParam {
  return typeof p === 'object' && p !== null
}

function processAsks(asks: [string, string][]): [number, number][] {
  const sorted = [...asks].sort((a, b) => parseFloat(a[0]) - parseFloat(b[0]))
  let cum = 0
  return sorted.map(([price, size]) => {
    cum += parseFloat(size)
    return [parseFloat(price), cum]
  })
}

function processBids(bids: [string, string][]): [number, number][] {
  const sorted = [...bids].sort((a, b) => parseFloat(b[0]) - parseFloat(a[0]))
  let cum = 0
  return sorted.map(([price, size]) => {
    cum += parseFloat(size)
    return [parseFloat(price), cum]
  })
}

function buildOption(
  asksData: [number, number][],
  bidsData: [number, number][],
): echarts.EChartsOption {
  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'cross',
        crossStyle: {
          color: 'var(--color-text-secondary)',
        },
      },
      formatter: (params: unknown) => {
        const lines: string[] = []
        const items = Array.isArray(params) ? params : []
        for (const p of items) {
          if (!isTooltipParam(p)) continue
          const price = p.data?.[0]
          const cum = p.data?.[1]
          if (price != null && cum != null) {
            lines.push(
              `${p.marker ?? ''} ${p.seriesName ?? ''}: Price ${price.toFixed(4)}, Cum ${cum.toFixed(4)}`,
            )
          }
        }
        return lines.join('<br>')
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
      axisLine: { lineStyle: { color: 'var(--color-text-secondary)' } },
      axisLabel: { color: 'var(--color-text-regular)' },
      splitLine: { lineStyle: { color: '#eee' } },
    },
    yAxis: {
      type: 'value',
      name: 'Cumulative Quantity',
      nameLocation: 'middle',
      nameGap: 45,
      axisLine: { lineStyle: { color: 'var(--color-text-secondary)' } },
      axisLabel: { color: 'var(--color-text-regular)' },
      splitLine: { lineStyle: { color: '#eee' } },
    },
    series: [
      {
        name: 'Asks',
        type: 'line',
        data: asksData,
        step: 'middle',
        lineStyle: { color: '#f56c6c', width: 1 },
        itemStyle: { color: '#f56c6c' },
        areaStyle: { color: 'rgba(245, 108, 108, 0.15)' },
        symbol: 'none',
      },
      {
        name: 'Bids',
        type: 'line',
        data: bidsData,
        step: 'middle',
        lineStyle: { color: '#67c23a', width: 1 },
        itemStyle: { color: '#67c23a' },
        areaStyle: { color: 'rgba(103, 194, 58, 0.15)' },
        symbol: 'none',
      },
    ],
  }
}

function updateChart() {
  const instance = chartInstance.value
  const book = rawOrderbook.value
  if (!instance || !book) return

  const asksData = processAsks(book.asks)
  const bidsData = processBids(book.bids)

  if (asksData.length === 0 && bidsData.length === 0) return

  instance.setOption(buildOption(asksData, bidsData), { notMerge: true })
}

function handleOrderbookUpdate(book: WsOrderBook) {
  const now = Date.now()
  if (now - lastUpdateTime.value < 500) {
    return
  }
  lastUpdateTime.value = now
  rawOrderbook.value = book
  updateChart()
}

async function setupListener() {
  unlistenFn = await listen<WsOrderBook>('ws:orderbook', (event) => {
    const data = event.payload
    if (data.inst_id !== props.symbol) return
    handleOrderbookUpdate(data)
  })
}

function initChart() {
  if (!chartRef.value) return
  if (chartInstance.value) {
    chartInstance.value.dispose()
    chartInstance.value = null
  }
  chartInstance.value = echarts.init(chartRef.value)
  const book = rawOrderbook.value
  if (book && (book.asks.length > 0 || book.bids.length > 0)) {
    const asksData = processAsks(book.asks)
    const bidsData = processBids(book.bids)
    chartInstance.value.setOption(buildOption(asksData, bidsData))
  }
}

onMounted(() => {
  initChart()
  setupListener()
})

onUnmounted(() => {
  if (unlistenFn) {
    unlistenFn()
    unlistenFn = null
  }
  chartInstance.value?.dispose()
  chartInstance.value = null
})
</script>

<template>
  <div class="order-book-depth">
    <div class="chart-header">
      <span class="title">Order Book Depth</span>
      <span class="symbol">{{ symbol }}</span>
    </div>
    <div ref="chartRef" class="chart-container"></div>
  </div>
</template>

<style scoped>
.order-book-depth {
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
  height: 400px;
  width: 100%;
}
</style>
