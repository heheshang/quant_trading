<template>
  <el-card class="okx-section-card">
    <template #header>
      <div class="card-header">
        <span>K 线</span>
      </div>
    </template>
    <el-form inline size="small">
      <el-form-item label="交易对">
        <el-select
          v-model="candleInstId"
          style="width: 160px"
          filterable
          @change="fetchCandles"
        >
          <el-option
            v-for="inst in instruments"
            :key="inst.instId"
            :label="inst.instId"
            :value="inst.instId"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="周期">
        <el-select
          v-model="candleBar"
          style="width: 100px"
          @change="fetchCandles"
        >
          <el-option label="1m" value="1m" />
          <el-option label="5m" value="5m" />
          <el-option label="15m" value="15m" />
          <el-option label="1H" value="1H" />
          <el-option label="4H" value="4H" />
          <el-option label="1D" value="1D" />
        </el-select>
      </el-form-item>
    </el-form>
    <div v-loading="loading" class="chart-wrapper">
      <div ref="candleChartRef" style="height: 300px" />
      <div v-if="candleError" class="market-data-placeholder">
        {{ candleError }}
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import * as echarts from 'echarts'
import { getOkxCandles } from '@/services/okx'
import type { OkxCandle } from '@/services/types'

interface Instrument {
  instId: string
}

withDefaults(
  defineProps<{
    instruments: Instrument[]
  }>(),
  {
    instruments: () => [],
  },
)

const candleInstId = ref('BTC-USDT')
const candleBar = ref('1H')
const candleChartRef = ref<HTMLDivElement>()
const candleError = ref('')
const loading = ref(false)

let chartInstance: echarts.ECharts | null = null

function formatTimestamp(ts: string): string {
  const d = new Date(Number(ts))
  const month = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  const hours = String(d.getHours()).padStart(2, '0')
  const minutes = String(d.getMinutes()).padStart(2, '0')
  return `${month}-${day} ${hours}:${minutes}`
}

function renderChart(candles: OkxCandle[]): void {
  if (!candleChartRef.value) return

  if (!chartInstance) {
    chartInstance = echarts.init(candleChartRef.value)
  }

  const sorted = [...candles].sort(
    (a, b) => Number(a.ts) - Number(b.ts),
  )
  const times = sorted.map((c) => formatTimestamp(c.ts))
  const values = sorted.map((c) => [c.o, c.c, c.l, c.h])

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
          color: '#67C23A',
          color0: '#F56C6C',
          borderColor: '#67C23A',
          borderColor0: '#F56C6C',
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
    const candles = await getOkxCandles(
      candleInstId.value,
      candleBar.value,
      60,
    )
    await nextTick()
    renderChart(candles)
  } catch (err: unknown) {
    candleError.value =
      err instanceof Error ? err.message : '获取K线失败'
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchCandles()
})

onUnmounted(() => {
  if (chartInstance) {
    chartInstance.dispose()
    chartInstance = null
  }
})

defineExpose({
  candleChartRef,
  candleError,
  fetchCandles,
})
</script>

<style scoped>
.okx-section-card {
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

.market-data-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  color: var(--color-text-secondary);
}
</style>
