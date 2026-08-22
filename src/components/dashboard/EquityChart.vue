<template>
  <el-card>
    <template #header>
      <div class="card-header"><span>资产曲线</span></div>
    </template>
    <div ref="chartRef" style="height: 400px"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import * as echarts from 'echarts'
import { useFormatting } from '@/composables/useFormatting'

const props = defineProps<{
  equityHistory?: [string, number][]
}>()

const chartRef = ref<HTMLDivElement>()
let chart: echarts.ECharts | null = null
let resizeHandler: (() => void) | null = null

const { formatCurrency } = useFormatting()

interface TooltipAxisParam {
  axisValue: string
  value: number
  seriesName: string
  color: string
}

function initChart() {
  if (!chartRef.value) return
  chart = echarts.init(chartRef.value)

  if (props.equityHistory && props.equityHistory.length > 0) {
    const dates = props.equityHistory.map(([d]) =>
      new Date(d).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }),
    )
    const values = props.equityHistory.map(([, v]) => v)
    chart.setOption({
      tooltip: {
        trigger: 'axis',
        formatter: (rawParams: object | object[]) => {
          const params = (Array.isArray(rawParams) ? rawParams : [rawParams]) as unknown as TooltipAxisParam[]
          return `${params[0].axisValue}<br/>¥${formatCurrency(params[0].value)}`
        },
      },
      xAxis: { type: 'category', data: dates },
      yAxis: {
        type: 'value',
        axisLabel: { formatter: (v: number) => '¥' + (v / 10000).toFixed(0) + '万' },
      },
      series: [
        {
          data: values,
          type: 'line',
          smooth: true,
          areaStyle: {},
          lineStyle: { width: 3 },
          itemStyle: { color: '#409EFF' },
        },
      ],
    })
  } else {
    chart.setOption({
      graphic: {
        elements: [{
          type: 'text',
          key: 'no-data',
          style: { text: '暂无资产历史', fontSize: 16, textAlign: 'center', fill: 'var(--color-text-secondary)' },
          position: ['50%', '50%'],
        }],
      },
    })
  }

  if (resizeHandler) window.removeEventListener('resize', resizeHandler)
  resizeHandler = () => chart?.resize()
  window.addEventListener('resize', resizeHandler)
}

watch(() => props.equityHistory, () => {
  nextTick(() => initChart())
})

onMounted(() => {
  nextTick(() => initChart())
})

onUnmounted(() => {
  if (resizeHandler) window.removeEventListener('resize', resizeHandler)
  resizeHandler = null
  chart?.dispose()
  chart = null
})
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
