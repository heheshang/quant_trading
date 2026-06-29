<template>
  <el-card class="risk-chart-card">
    <template #header>
      <div class="card-header">
        <span>风险指标趋势</span>
      </div>
    </template>
    <div ref="chartRef" style="height: 300px;"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'

const chartRef = ref<HTMLDivElement | null>(null)
let riskChart: echarts.ECharts | null = null
let resizeHandler: (() => void) | null = null

function initRiskChart() {
  const dom = chartRef.value
  if (!dom) return
  riskChart = echarts.init(dom)
  riskChart.setOption({
    title: {
      text: '暂无历史数据',
      left: 'center',
      top: 'center',
      textStyle: { color: '#909399', fontSize: 14, fontWeight: 'normal' },
    },
    xAxis: { type: 'category', data: [] },
    yAxis: {
      type: 'value',
      axisLabel: { formatter: (v: number) => (v * 100).toFixed(0) + '%' },
    },
    grid: { left: 60, right: 20, bottom: 40, top: 20 },
    series: [],
  })
  resizeHandler = () => riskChart?.resize()
  window.addEventListener('resize', resizeHandler)
}

onMounted(() => {
  initRiskChart()
})

onUnmounted(() => {
  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler)
    resizeHandler = null
  }
  riskChart?.dispose()
  riskChart = null
})
</script>

<style scoped>
.risk-chart-card {
  margin-bottom: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
