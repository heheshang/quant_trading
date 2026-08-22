import * as echarts from 'echarts/core'
import type { LineSeriesOption } from 'echarts/charts'
import { useChartTheme } from './useChartTheme'

/**
 * Data point shape consumed by the performance chart.
 */
export interface ChartPoint {
  date: string | number
  value: number
  volume?: number
}

/**
 * Calculate running drawdown (in percent) given a value series.
 * For each point, drawdown = (current - running_max) / running_max * 100.
 */
export function calculateDrawdown(data: number[]): number[] {
  const drawdown: number[] = []
  let maxValue = data[0]
  for (let i = 0; i < data.length; i++) {
    maxValue = Math.max(maxValue, data[i])
    const drawdownValue = (data[i] - maxValue) / maxValue * 100
    drawdown.push(drawdownValue)
  }
  return drawdown
}

/**
 * Build ECharts options for the performance chart.
 *
 * @param data        - series of {date, value} points (date may be a string or numeric index)
 * @param dataSource  - when 'backtest', also renders a drawdown series
 */
export function buildChartOptions(
  data: ChartPoint[],
  dataSource: 'backtest' | 'realtime',
): echarts.EChartsCoreOption {
  if (!data.length) return {}
  // React to the app theme so axis/text/tooltip adapt to dark mode.
  const theme = useChartTheme().palette.value

  const dates = data.map((item) =>
    typeof item.date === 'string' ? item.date : `Day ${item.date}`,
  )
  const values = data.map((item) => item.value)
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
      lineStyle: { width: 2, color: '#67C23A' },
      itemStyle: { color: '#67C23A' },
      areaStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
          { offset: 0, color: 'rgba(103, 194, 58, 0.3)' },
          { offset: 1, color: 'rgba(103, 194, 58, 0.1)' },
        ]),
      },
      markLine: {
        silent: true,
        data: [{ type: 'max', name: '最高值', itemStyle: { color: '#67C23A' } }],
      },
    },
  ]

  if (dataSource === 'backtest') {
    series.push({
      name: '最大回撤',
      data: calculateDrawdown(values),
      type: 'line',
      smooth: 0.3,
      symbol: 'none',
      lineStyle: { width: 1, color: '#F56C6C', type: 'dashed' },
      itemStyle: { color: '#F56C6C' },
      markLine: {
        silent: true,
        data: [{ type: 'min', name: '最低值', itemStyle: { color: '#F56C6C' } }],
      },
    })
  }

  return {
    title: {
      text: '策略绩效图表',
      left: 'center',
      top: 0,
      textStyle: { fontSize: 14, color: theme.axisLabel },
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: theme.tooltipBg,
      borderColor: theme.tooltipBorder,
      borderWidth: 1,
      textStyle: { fontSize: 12, color: theme.tooltipText },
      formatter: (params: any) => {
        const date = params[0].name
        const value = params[0].value
        return `${date}<br/>${params[0].seriesName}: ${value.toFixed(2)}`
      },
    },
    legend: {
      data: ['权益曲线', '最大回撤'],
      top: 30,
      textStyle: { fontSize: 12 },
    },
    grid: { left: '3%', right: '4%', bottom: '8%', top: '60px', containLabel: true },
    xAxis: {
      type: 'category',
      data: dates,
      axisLabel: { rotate: 45, fontSize: 10, color: theme.axisLabel },
      axisLine: { lineStyle: { color: theme.splitLine } },
    },
    yAxis: {
      type: 'value',
      min: minValue - range * 0.1,
      max: maxValue + range * 0.1,
      axisLabel: { formatter: (value: number) => value.toFixed(2), color: theme.axisLabel },
      splitLine: { lineStyle: { color: theme.splitLine } },
    },
    dataZoom: [
      { type: 'inside', start: 0, end: 100 },
      { type: 'slider', start: 0, end: 100, bottom: 20 },
    ],
    series,
  }
}
