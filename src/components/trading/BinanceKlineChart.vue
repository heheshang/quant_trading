<template>
  <el-card class="binance-kline-chart" shadow="never">
    <template #header>
      <div class="card-header">
        <span>K 线图</span>
      </div>
    </template>
    <el-form inline size="small">
      <el-form-item label="交易对">
        <el-input v-model="chartSymbol" style="width: 160px" @keyup.enter="onIntervalChange" />
      </el-form-item>
      <el-form-item label="周期">
        <el-select v-model="chartInterval" style="width: 100px" @change="onIntervalChange">
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
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import * as echarts from 'echarts'
import { getKlines } from '@/services/market'
import { startBinanceMarketData, subscribeBinanceCandle } from '@/services/binance'
import type { BinanceKline, MarketDataRecord } from '@/services/types'
import { getChartSeriesColors } from '@/composables/useChartTheme'

const props = defineProps<{ symbol?: string }>()
const emit = defineEmits<{ (e: 'update:symbol', v: string): void }>()
/** 由父级驱动（交易面板选中标的）；图表内改动回传 domain 符号。 */
const chartSymbol = computed({
  get: () => props.symbol ?? 'BTCUSDT',
  set: (v: string) => emit('update:symbol', toDomainSymbol(v)),
})
const chartInterval = ref('1h')
const candles = ref<BinanceKline[]>([])
const candleError = ref('')
const loading = ref(false)

const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null
let pollTimer: ReturnType<typeof setInterval> | null = null

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

/** 币安符号 → domain（'BTCUSDT'→'BTC-USDT'），DB `market_data` 用 domain 作 instrument_id。 */
function toDomainSymbol(sym: string): string {
  if (sym.includes('-')) return sym
  return sym.replace(/(USDT|USDC|BUSD|TUSD|FDUSD|DAI)$/, '-$1')
}

/** DB K 线行 → chart 使用的 `BinanceKline`。 */
function dbKlineToBinance(r: MarketDataRecord): BinanceKline {
  const openTime = new Date(r.timestamp).getTime()
  const intervalMs = 3_600_000 // 默认按小时；短周期误差可接受
  return {
    open_time: openTime,
    open: r.open,
    high: r.high,
    low: r.low,
    close: r.close,
    volume: r.volume,
    close_time: openTime + intervalMs,
    quote_volume: 0,
    trades: 0,
  }
}

async function fetchCandles(): Promise<void> {
  candleError.value = ''
  loading.value = true
  try {
    // 从 DB 读（remote WS 已导入 `market_data`），符合「前端走 db」。
    const rows = await getKlines(toDomainSymbol(chartSymbol.value), chartInterval.value, 60)
    candles.value = rows.map(dbKlineToBinance)
    await nextTick()
    renderChart()
  } catch (err: unknown) {
    candleError.value =
      err instanceof Error ? err.message : '获取 K 线失败'
  } finally {
    loading.value = false
  }
}

/** 确保后端 WS 已在运行并订阅当前周期（驱动 remote WS → DB 导入当前周期）。 */
async function subscribeCurrentInterval(): Promise<void> {
  // 幂等启动：已在运行时返回 CONFLICT，忽略错误以便继续订阅。
  await startBinanceMarketData().catch(() => {})
  try {
    await subscribeBinanceCandle(toDomainSymbol(chartSymbol.value), chartInterval.value)
    candleError.value = ''
  } catch (e) {
    candleError.value = e instanceof Error ? e.message : '订阅实时行情失败'
  }
}

/** 周期/标的切换：重订阅并刷新。 */
async function onIntervalChange(): Promise<void> {
  await subscribeCurrentInterval()
  await fetchCandles()
}

// 父级切换标的（交易面板选中的 symbol）时，重订阅并刷新该图表。
watch(
  () => props.symbol,
  () => {
    if (props.symbol) void onIntervalChange()
  },
)

onMounted(async () => {
  await subscribeCurrentInterval()
  await fetchCandles()
  if (pollTimer) clearInterval(pollTimer)
  pollTimer = setInterval(fetchCandles, 5000)
})

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  chartInstance?.dispose()
  chartInstance = null
})

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
