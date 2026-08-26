<template>
  <el-card>
    <template #header>
      <div class="card-header"><span>持仓分布</span></div>
    </template>
    <div ref="chartRef" style="height: 400px"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { EChartsCoreOption } from 'echarts/core'
import { useEcharts } from '@/composables/useEcharts'
import { useFormatting } from '@/composables/useFormatting'
import type { Position } from '@/services/types'
import { useChartTheme } from '@/composables/useChartTheme'

const props = defineProps<{
  positions: Position[]
}>()

const chartRef = ref<HTMLDivElement>()

const { formatCurrency } = useFormatting()

interface TooltipItemParam {
  name: string
  value: number
  percent: number
  color: string
}

const chartOptions = computed<EChartsCoreOption>(() => {
  const theme = useChartTheme().palette.value

  if (props.positions.length === 0) {
    return {
      graphic: {
        elements: [{
          type: 'text',
          key: 'no-data',
          style: { text: '暂无持仓', fontSize: 16, textAlign: 'center', fill: theme.axisLabel },
          left: 'center',
          top: 'middle',
        }],
      },
    }
  }

  return {
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
  }
})

useEcharts(chartRef, chartOptions)
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
