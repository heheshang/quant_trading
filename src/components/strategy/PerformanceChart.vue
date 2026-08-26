<template>
  <div class="performance-chart" ref="chartRef">
    <div v-if="loading" class="chart-loading">
      <el-skeleton animated />
    </div>
    <div v-else-if="error" class="chart-error">
      <el-result
        icon="warning"
        :title="errorTitle"
        :sub-title="errorSubTitle"
      />
    </div>
    <div v-else ref="echartRef" class="echart-container"></div>

    <ChartControls
      :time-range="timeRange"
      :data-source="selectedDataSource"
      :show-controls="showControls"
      @update:time-range="timeRange = $event"
      @update:data-source="selectedDataSource = $event"
      @change="onControlsChange"
    />

    <div class="chart-legend" :style="{ background: isDark ? 'rgba(29,30,31,0.92)' : 'rgba(255,255,255,0.92)' }">
      <div class="legend-item">
        <span class="legend-color equity-curve"></span>
        <span>权益曲线</span>
      </div>
      <div class="legend-item">
        <span class="legend-color drawdown"></span>
        <span>最大回撤</span>
      </div>
      <div class="legend-item">
        <span class="legend-color sharpe"></span>
        <span>夏普比率</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'
import { LineChart } from 'echarts/charts'
import {
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  MarkLineComponent,
} from 'echarts/components'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import * as echarts from 'echarts/core'
import { ElSkeleton, ElResult } from 'element-plus'

import { useEcharts } from '@/composables/useEcharts'
import { buildChartOptions } from '@/composables/usePerformanceChartOptions'
import { useChartTheme } from '@/composables/useChartTheme'
import ChartControls from './ChartControls.vue'

use([
  LineChart,
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  MarkLineComponent,
  CanvasRenderer,
])

const props = withDefaults(
  defineProps<{
    equityCurve?: number[]
    marketData?: { date: string | number; value: number; volume?: number }[]
    dataSource?: 'backtest' | 'realtime'
    height?: string
    showControls?: boolean
  }>(),
  {
    equityCurve: () => [],
    marketData: () => [],
    dataSource: 'backtest',
    height: '400px',
    showControls: true,
  },
)

type ChartInstance = ReturnType<typeof echarts.init>

const emit = defineEmits<{
  'data-change': [data: unknown]
  'chart-ready': [chart: ChartInstance | null]
}>()

const chartRef = ref<HTMLDivElement>()
const echartRef = ref<HTMLDivElement>()
const loading = ref(true)
const error = ref(false)
const errorTitle = ref('加载失败')
const errorSubTitle = ref('请稍后重试')
const timeRange = ref('1M')
const selectedDataSource = ref(props.dataSource)

const { isDark } = useChartTheme()

const chartData = computed(() => {
  if (selectedDataSource.value === 'backtest' && props.equityCurve.length > 0) {
    return props.equityCurve.map((value, index) => ({ date: index, value }))
  } else if (props.marketData.length > 0) {
    return props.marketData
  }
  return []
})

const chartOptions = computed(() =>
  buildChartOptions(chartData.value, selectedDataSource.value),
)

const { instance: chartInstance, resize } = useEcharts(echartRef, chartOptions)

function onControlsChange() {
  nextTick(resize)
}

watch(chartInstance, (inst) => {
  if (inst) {
    loading.value = false
    error.value = false
    emit('chart-ready', inst as ChartInstance)
  }
})

</script>

<style scoped>
.performance-chart {
  position: relative;
  width: 100%;
  height: v-bind(height);
}

.chart-loading,
.chart-error {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 300px;
}

.echart-container {
  width: 100%;
  height: 100%;
}

.chart-legend {
  position: absolute;
  bottom: 10px;
  left: 10px;
  z-index: 10;
  background: rgba(255, 255, 255, 0.9);
  padding: 8px 12px;
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  display: flex;
  gap: 16px;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
}

.legend-color {
  width: 12px;
  height: 12px;
  border-radius: 2px;
}

.legend-color.equity-curve { background-color: var(--color-success); }
.legend-color.drawdown { background-color: var(--color-danger); }
.legend-color.sharpe { background-color: var(--color-primary); }
</style>
