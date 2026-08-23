<template>
  <el-card class="binance-kline-chart" shadow="never">
    <template #header>
      <div class="card-header">
        <span>K 线图</span>
      </div>
    </template>
    <el-form inline size="small">
      <el-form-item label="交易对">
        <el-input v-model="chartSymbol" style="width: 160px" @keyup.enter="fetchCandles" />
      </el-form-item>
      <el-form-item label="周期">
        <el-select v-model="chartInterval" style="width: 100px" @change="fetchCandles">
          <el-option label="1m" value="1m" />
          <el-option label="5m" value="5m" />
          <el-option label="15m" value="15m" />
          <el-option label="1h" value="1h" />
          <el-option label="4h" value="4h" />
          <el-option label="1d" value="1d" />
        </el-select>
      </el-form-item>
      <el-button size="small" :loading="loading" @click="fetchCandles">刷新</el-button>
    </el-form>
    <div v-loading="loading" class="chart-wrapper">
      <div ref="chartRef" style="height: 300px" />
      <p v-if="candleError" class="chart-hint">{{ candleError }}</p>
      <p v-else-if="!candles.length" class="chart-hint">暂无 K 线数据</p>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import * as echarts from 'echarts'
import { getBinanceCandles } from '@/services/binance'
import type { BinanceKline } from '@/services/types'
import { getChartSeriesColors } from '@/composables/useChartTheme'

const chartSymbol = ref('BTCUSDT')
const chartInterval = ref('1h')
const candles = ref<BinanceKline[]>([])
const candleError = ref('')
const loading = ref(false)

const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null

function formatTimestamp(ms: number): string {
  const d = new Date(ms)
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  const hours = String(d.getHours()).padStart(2, '0')
  const minutes = String(d.getMinutes()).padStart(2, '0')
  return `${month}-${day} ${hours}:${minutes}`
}

function renderChart(): void {
  if (!chartRef.value) return
  if (!chartInstance) {
    chartInstance = echarts.init(chartRef.value)
  }
  const chartColors = getChartSeriesColors()
  const upColor = chartColors.green
  const downColor = chartColors.red

  const sorted = [...candles.value].sort((a, b) => a.open_time - b.open_time)
  const times = sorted.map((c) => formatTimestamp(c.open_time))
  const values = sorted.map((c) => [c.open, c.close, c.low, c.high])

  chartInstance.setOption({
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'cross' },
    },
    xAxis: {
      type: 'category',
      data: times,
      axisLabel: { rotate: 45 },
    },
    yAxis: {
      type: 'value',
      scale: true,
    },
    series: [
      {
        type: 'candlestick',
        data: values,
        itemStyle: {
          color: upColor,
          color0: downColor,
          borderColor: upColor,
          borderColor0: downColor,
        },
      },
    ],
    grid: {
      left: '5%',
      right: '5%',
      bottom: '5%',
      top: '5%',
    },
  })
}

async function fetchCandles(): Promise<void> {
  candleError.value = ''
  loading.value = true
  try {
    candles.value = await getBinanceCandles(chartSymbol.value, chartInterval.value, 60)
    await nextTick()
    renderChart()
  } catch (err: unknown) {
    candleError.value =
      err instanceof Error ? err.message : '获取 K 线失败'
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchCandles()
})

onUnmounted(() => {
  chartInstance?.dispose()
  chartInstance = null
})

defineExpose({ chartRef, candleError, candles, fetchCandles })
</script>

<style scoped>
.binance-kline-chart {
  margin-bottom: 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chart-wrapper {
  position: relative;
  min-height: 300px;
}

.chart-hint {
  margin: 0;
  padding: 12px 0;
  color: var(--color-text-secondary);
  text-align: center;
  font-size: 13px;
}
</style>
