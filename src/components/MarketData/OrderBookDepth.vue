<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick, computed } from 'vue'
import * as echarts from 'echarts'
import { useMarketData } from '@/composables/useMarketData'

const props = defineProps<{
  symbol: string
}>()

const { orderbook } = useMarketData()

const chartRef = ref<HTMLDivElement>()
let chart: echarts.ECharts | null = null

let updateTimer: ReturnType<typeof setTimeout> | null = null

const currentBook = computed(() => orderbook.value[props.symbol])

function processData(book: typeof currentBook.value) {
  if (!book || !book.asks || !book.bids) {
    return { asksData: [], bidsData: [] }
  }

  let askCum = 0
  const asksData = book.asks.map(([price, size]) => {
    askCum += parseFloat(size)
    return [parseFloat(price), askCum]
  })

  let bidCum = 0
  const bidsData = book.bids.map(([price, size]) => {
    bidCum += parseFloat(size)
    return [parseFloat(price), bidCum]
  })

  return { asksData, bidsData }
}

function buildOption(
  asksData: number[][],
  bidsData: number[][],
): echarts.EChartsOption {
  return {
    tooltip: {
      trigger: 'axis',
      axisPointer: {
        type: 'cross',
        crossStyle: { color: '#999' },
      },
      formatter: (params: any) => {
        const lines: string[] = []
        for (const p of params) {
          const price = p.data?.[0]
          const size = p.data?.[1]
          if (price != null && size != null) {
            lines.push(`${p.marker} ${p.seriesName}: Price ${price}, Cum ${size.toFixed(4)}`)
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
      axisLine: { lineStyle: { color: '#999' } },
      axisLabel: { color: '#666' },
      splitLine: { lineStyle: { color: '#eee' } },
    },
    yAxis: {
      type: 'value',
      name: 'Cumulative Size',
      nameLocation: 'middle',
      nameGap: 45,
      axisLine: { lineStyle: { color: '#999' } },
      axisLabel: { color: '#666' },
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
  if (!chart) return
  const { asksData, bidsData } = processData(currentBook.value)
  if (asksData.length === 0 && bidsData.length === 0) return
  chart.setOption(buildOption(asksData, bidsData), { notMerge: true })
}

function scheduleUpdate() {
  if (updateTimer) return
  updateTimer = setTimeout(() => {
    updateTimer = null
    updateChart()
  }, 500)
}

function initChart() {
  if (!chartRef.value) return
  if (chart) {
    chart.dispose()
    chart = null
  }
  chart = echarts.init(chartRef.value)
  const { asksData, bidsData } = processData(currentBook.value)
  if (asksData.length > 0 || bidsData.length > 0) {
    chart.setOption(buildOption(asksData, bidsData))
  }
}

watch(currentBook, () => {
  scheduleUpdate()
})

watch(() => props.symbol, () => {
  nextTick(() => initChart())
})

onMounted(() => {
  nextTick(() => initChart())
})

onUnmounted(() => {
  if (updateTimer) {
    clearTimeout(updateTimer)
    updateTimer = null
  }
  chart?.dispose()
  chart = null
})
</script>

<template>
  <div>
    <div
      v-if="!currentBook || !currentBook.asks || !currentBook.bids || currentBook.asks.length === 0 || currentBook.bids.length === 0"
      style="height: 400px; display: flex; align-items: center; justify-content: center; color: #999;"
    >
      Waiting for data...
    </div>
    <div
      v-show="currentBook && currentBook.asks && currentBook.bids && currentBook.asks.length > 0 && currentBook.bids.length > 0"
      ref="chartRef"
      style="height: 400px;"
    ></div>
  </div>
</template>
