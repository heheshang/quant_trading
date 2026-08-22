<template>
  <el-card>
    <template #header>
      <div class="card-header"><span>持仓分布</span></div>
    </template>
    <div ref="chartRef" style="height: 400px"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import * as echarts from 'echarts'
import { useFormatting } from '@/composables/useFormatting'
import type { Position } from '@/services/types'

const props = defineProps<{
  positions: Position[]
}>()

const chartRef = ref<HTMLDivElement>()
let chart: echarts.ECharts | null = null
let resizeHandler: (() => void) | null = null

const { formatCurrency } = useFormatting()

interface TooltipItemParam {
  name: string
  value: number
  percent: number
  color: string
}

function initChart() {
  if (!chartRef.value) return
  chart = echarts.init(chartRef.value)

  if (props.positions.length > 0) {
    chart.setOption({
      tooltip: {
        trigger: 'item',
        formatter: (rawParams: object | object[]) => {
          const params = (Array.isArray(rawParams) ? rawParams[0] : rawParams) as unknown as TooltipItemParam
          return `${params.name}<br/>¥${formatCurrency(params.value)} (${params.percent}%)`
        },
      },
      series: [
        {
          type: 'pie',
          radius: ['40%', '70%'],
          data: props.positions.map((pos) => ({
            value: Number(pos.market_value),
            name: pos.symbol,
          })),
          emphasis: {
            itemStyle: { shadowBlur: 10, shadowOffsetX: 0, shadowColor: 'rgba(0, 0, 0, 0.5)' },
          },
        },
      ],
    })
  } else {
    chart.setOption({
      graphic: {
        elements: [{
          type: 'text',
          key: 'no-data',
          style: { text: '暂无持仓', fontSize: 16, textAlign: 'center', fill: 'var(--color-text-secondary)' },
          position: ['50%', '50%'],
        }],
      },
    })
  }

  if (resizeHandler) window.removeEventListener('resize', resizeHandler)
  resizeHandler = () => chart?.resize()
  window.addEventListener('resize', resizeHandler)
}

watch(() => props.positions, () => {
  nextTick(() => initChart())
}, { deep: true })

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
