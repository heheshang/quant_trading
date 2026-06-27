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
    
    <div class="chart-controls" v-if="showControls">
      <el-radio-group v-model="timeRange" size="small">
        <el-radio-button label="1D">1D</el-radio-button>
        <el-radio-button label="1W">1W</el-radio-button>
        <el-radio-button label="1M">1M</el-radio-button>
        <el-radio-button label="3M">3M</el-radio-button>
        <el-radio-button label="1Y">1Y</el-radio-button>
      </el-radio-group>
      
      <el-select v-model="selectedDataSource" size="small" style="width: 120px">
        <el-option label="回测数据" value="backtest" />
        <el-option label="实时数据" value="realtime" />
      </el-select>
    </div>
    
    <div class="chart-legend">
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
import { ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue'
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { 
  TitleComponent, 
  TooltipComponent, 
  LegendComponent, 
  GridComponent,
  DataZoomComponent,
  MarkLineComponent
} from 'echarts/components'
import type {
  LineSeriesOption,
} from 'echarts/charts'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { ElSkeleton, ElResult } from 'element-plus'


use([
  LineChart,
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent,
  DataZoomComponent,
  MarkLineComponent,
  CanvasRenderer
])

const props = withDefaults(
  defineProps<{
    equityCurve?: number[]
    marketData?: {
      date: string | number
      value: number
      volume?: number
    }[]
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
  }
)

const emit = defineEmits<{
  'data-change': [data: any]
  'chart-ready': [chart: any]
}>()

const chartRef = ref<HTMLDivElement>()
const echartRef = ref<HTMLDivElement>()
const chartInstance = ref<any>(null)
const loading = ref(true)
const error = ref(false)
const errorTitle = ref('加载失败')
const errorSubTitle = ref('请稍后重试')
const timeRange = ref('1M')
const selectedDataSource = ref(props.dataSource)

const chartData = computed(() => {
  if (selectedDataSource.value === 'backtest' && props.equityCurve.length > 0) {
    return props.equityCurve.map((value, index) => ({
      date: index,
      value: value,
    }))
  } else if (props.marketData.length > 0) {
    return props.marketData
  }
  return []
})

const chartOptions = computed(() => {
  if (!chartData.value.length) return {}
  
  const dates = chartData.value.map(item => {
    if (typeof item.date === 'string') {
      return item.date
    }
    return `Day ${item.date}`
  })
  
  const values = chartData.value.map(item => item.value)
  
  const maxValue = Math.max(...values)
  const minValue = Math.min(...values)
  const range = maxValue - minValue
  
  const series: LineSeriesOption[] = [
    {
      name: '权益曲线',
      data: values,
      type: 'line',
      smooth: 0.3,
      symbol: 'none',
      lineStyle: {
        width: 2,
        color: '#67C23A',
      },
      itemStyle: {
        color: '#67C23A',
      },
      areaStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
          { offset: 0, color: 'rgba(103, 194, 58, 0.3)' },
          { offset: 1, color: 'rgba(103, 194, 58, 0.1)' }
        ])
      },
      markLine: {
        silent: true,
        data: [{ type: 'max', name: '最高值', itemStyle: { color: '#67C23A' } }]
      }
    }
  ]
  
  if (selectedDataSource.value === 'backtest') {
    const drawdown = calculateDrawdown(values)
    series.push({
      name: '最大回撤',
      data: drawdown,
      type: 'line',
      smooth: 0.3,
      symbol: 'none',
      lineStyle: {
        width: 1,
        color: '#F56C6C',
        type: 'dashed'
      },
      itemStyle: {
        color: '#F56C6C',
      },
      markLine: {
        silent: true,
        data: [{ type: 'min', name: '最低值', itemStyle: { color: '#F56C6C' } }]
      }
    })
  }
  
  return {
    title: {
      text: '策略绩效图表',
      left: 'center',
      top: 0,
      textStyle: {
        fontSize: 14,
        color: '#606266'
      }
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(255, 255, 255, 0.9)',
      borderColor: '#EBEEF5',
      borderWidth: 1,
      textStyle: {
        fontSize: 12
      },
      formatter: (params: any) => {
        const date = params[0].name
        const value = params[0].value
        return `${date}<br/>${params[0].seriesName}: ${value.toFixed(2)}`
      }
    },
    legend: {
      data: ['权益曲线', '最大回撤'],
      top: 30,
      textStyle: {
        fontSize: 12
      }
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '8%',
      top: '60px',
      containLabel: true
    },
    xAxis: {
      type: 'category',
      data: dates,
      axisLabel: {
        rotate: 45,
        fontSize: 10
      }
    },
    yAxis: {
      type: 'value',
      min: minValue - range * 0.1,
      max: maxValue + range * 0.1,
      axisLabel: {
        formatter: (value: number) => value.toFixed(2)
      }
    },
    dataZoom: [
      {
        type: 'inside',
        start: 0,
        end: 100
      },
      {
        type: 'slider',
        start: 0,
        end: 100,
        bottom: 20
      }
    ],
    series: series
  }
})

function calculateDrawdown(data: number[]): number[] {
  const drawdown: number[] = []
  let maxValue = data[0]
  
  for (let i = 0; i < data.length; i++) {
    maxValue = Math.max(maxValue, data[i])
    const drawdownValue = (data[i] - maxValue) / maxValue * 100
    drawdown.push(drawdownValue)
  }
  
  return drawdown
}

function initChart() {
  if (!echartRef.value) return
  
  try {
    loading.value = true
    error.value = false
    
    chartInstance.value = echarts.init(echartRef.value, {
      renderer: 'canvas',
      useDirtyRect: false
    })
    
    chartInstance.value.setOption(chartOptions.value)
    
    chartInstance.value.on('updateLayout', () => {
      chartInstance.value?.resize()
    })
    
    emit('chart-ready', chartInstance.value)
    
  } catch (err) {
    error.value = true
    errorTitle.value = '图表初始化失败'
    errorSubTitle.value = '请刷新页面重试'
    console.error('ECharts 初始化失败:', err)
  } finally {
    loading.value = false
  }
}

function disposeChart() {
  if (chartInstance.value) {
    chartInstance.value.dispose()
    chartInstance.value = null
  }
}

function handleResize() {
  chartInstance.value?.resize()
}

onMounted(() => {
  nextTick(() => {
    initChart()
    
    window.addEventListener('resize', handleResize)
  })
})

onUnmounted(() => {
  disposeChart()
  window.removeEventListener('resize', handleResize)
})

watch(() => [props.equityCurve, props.marketData, selectedDataSource.value], () => {
  if (chartInstance.value) {
    chartInstance.value.setOption(chartOptions.value)
  }
})

watch(timeRange, () => {
  if (chartInstance.value) {
    chartInstance.value.setOption(chartOptions.value)
  }
})
</script>

<style scoped>
.performance-chart {
  position: relative;
  width: 100%;
  height: v-bind(height);
}

.chart-loading, .chart-error {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 300px;
}

.echart-container {
  width: 100%;
  height: 100%;
}

.chart-controls {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 10;
  display: flex;
  gap: 8px;
  align-items: center;
  background: rgba(255, 255, 255, 0.9);
  padding: 4px 8px;
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
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

.legend-color.equity-curve {
  background-color: #67C23A;
}

.legend-color.drawdown {
  background-color: #F56C6C;
}

.legend-color.sharpe {
  background-color: #409EFF;
}
</style>
