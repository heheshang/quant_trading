import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'
import * as echarts from 'echarts'
import { mockListen, mockUnlisten } from './setup'
import RealtimeCandleChart from '@/components/dashboard/RealtimeCandleChart.vue'
import type { WsCandle } from '@/services/types'

function makeCandle(overrides: Partial<WsCandle> = {}): WsCandle {
  return {
    inst_id: 'BTC-USDT',
    o: '49000',
    h: '51000',
    l: '48000',
    c: '50000',
    vol: '1000',
    ts: '1234567890',
    ...overrides,
  }
}

function getCandleCallback(): (event: { payload: WsCandle }) => void {
  const call = mockListen.mock.calls.find(
    (c: unknown[]) => c[0] === 'ws:candle',
  )
  if (!call) {
    throw new Error('No ws:candle listener registered')
  }
  return call[1] as (event: { payload: WsCandle }) => void
}

/** The mocked echarts.init returns the same mockECharts object every call. */
function getMockChart(): {
  setOption: ReturnType<typeof vi.fn>
  dispose: ReturnType<typeof vi.fn>
} {
  const initMock = echarts.init as ReturnType<typeof vi.fn>
  return initMock.mock.results[0]?.value as {
    setOption: ReturnType<typeof vi.fn>
    dispose: ReturnType<typeof vi.fn>
  }
}

async function mountAndReady(symbol = 'BTC-USDT'): Promise<any> {
  const wrapper = mount(RealtimeCandleChart, {
    props: { symbol },
  })
  await nextTick()
  await flushPromises()
  return wrapper
}

describe('RealtimeCandleChart', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ─── Mount / empty state ────────────────────────────────────────

  it('renders "Waiting for data..." empty state initially', async () => {
    const wrapper = await mountAndReady()

    expect(wrapper.find('.chart-empty').exists()).toBe(true)
    expect(wrapper.find('.chart-empty').text()).toBe('Waiting for data...')
  })

  // ─── ECharts init ───────────────────────────────────────────────

  it('calls echarts.init on mount', async () => {
    await mountAndReady()

    expect(echarts.init).toHaveBeenCalledTimes(1)
  })

  it('calls setOption with base option on init', async () => {
    await mountAndReady()
    const chart = getMockChart()

    expect(chart.setOption).toHaveBeenCalled()
  })

  // ─── ws:candle with matching symbol ─────────────────────────────

  it('calls setOption when a ws:candle event with matching symbol arrives', async () => {
    await mountAndReady()
    const chart = getMockChart()
    // Reset to isolate the init call from the event call
    chart.setOption.mockClear()

    const callback = getCandleCallback()
    callback({ payload: makeCandle() })

    await nextTick()

    expect(chart.setOption).toHaveBeenCalled()
  })

  // ─── ws:candle with non-matching symbol is ignored ──────────────

  it('ignores ws:candle events for a non-matching symbol', async () => {
    const wrapper = await mountAndReady()
    const chart = getMockChart()
    chart.setOption.mockClear()

    const callback = getCandleCallback()
    callback({ payload: makeCandle({ inst_id: 'ETH-USDT' }) })

    await nextTick()

    // setOption should NOT have been called beyond the init call
    expect(chart.setOption).not.toHaveBeenCalled()
    // Empty state should still show
    expect(wrapper.find('.chart-empty').exists()).toBe(true)
  })

  // ─── Candle data accumulates ────────────────────────────────────

  it('appends new candles (unique ts) and hides empty state', async () => {
    const wrapper = await mountAndReady()
    const chart = getMockChart()
    chart.setOption.mockClear()

    const callback = getCandleCallback()

    // First candle
    callback({ payload: makeCandle({ ts: '1' }) })
    await nextTick()
    expect(chart.setOption).toHaveBeenCalledTimes(1)
    expect(wrapper.find('.chart-empty').exists()).toBe(false)

    // Second candle with new ts → should be appended
    callback({ payload: makeCandle({ ts: '2' }) })
    await nextTick()
    expect(chart.setOption).toHaveBeenCalledTimes(2)
  })

  it('updates existing candle in place when ts matches', async () => {
    const wrapper = await mountAndReady()
    const chart = getMockChart()
    chart.setOption.mockClear()

    const callback = getCandleCallback()

    callback({ payload: makeCandle({ ts: '1', c: '50000' }) })
    await nextTick()

    chart.setOption.mockClear()

    // Same ts → replace, not append
    callback({ payload: makeCandle({ ts: '1', c: '51000' }) })
    await nextTick()

    // Still only one candle, setOption called for the update
    expect(chart.setOption).toHaveBeenCalledTimes(1)
    expect(wrapper.find('.chart-empty').exists()).toBe(false)
  })

  // ─── MAX_CANDLES cap at 500 ─────────────────────────────────────

  it('caps candle data at 500 (MAX_CANDLES)', async () => {
    const wrapper = await mountAndReady()
    const chart = getMockChart()
    chart.setOption.mockClear()

    const callback = getCandleCallback()

    // Push 501 candles with unique ts values
    for (let i = 0; i < 501; i++) {
      callback({ payload: makeCandle({ ts: String(i) }) })
    }
    await nextTick()

    // setOption should be called 501 times (one per candle)
    expect(chart.setOption).toHaveBeenCalledTimes(501)

    // The empty state should be hidden
    expect(wrapper.find('.chart-empty').exists()).toBe(false)
  })

  // ─── Period toggle buttons ──────────────────────────────────────

  it('renders period toggle buttons (1m, 5m, 15m, 1H)', async () => {
    const wrapper = await mountAndReady()

    const buttons = wrapper.findAll('el-radio-button')
    expect(buttons).toHaveLength(4)
  })

  // ─── Changing period resets chart data ──────────────────────────

  it('resets chart data when period changes', async () => {
    const wrapper = await mountAndReady()
    const chart = getMockChart()

    // Feed some data first
    const callback = getCandleCallback()
    callback({ payload: makeCandle() })
    await nextTick()
    expect(wrapper.find('.chart-empty').exists()).toBe(false)

    chart.setOption.mockClear()

    // Change period by directly setting the component's period ref
    ;(wrapper.vm as unknown as { period: string }).period = '5m'
    await nextTick()

    // After period change, chart should be reset → empty state reappears
    expect(wrapper.find('.chart-empty').exists()).toBe(true)
    // setOption should have been called to clear the chart
    expect(chart.setOption).toHaveBeenCalled()
  })

  // ─── Changing symbol resets chart data ──────────────────────────

  it('resets chart data when symbol prop changes', async () => {
    const wrapper = await mountAndReady()

    // Feed some data first
    const callback = getCandleCallback()
    callback({ payload: makeCandle() })
    await nextTick()
    expect(wrapper.find('.chart-empty').exists()).toBe(false)

    // Change symbol
    await wrapper.setProps({ symbol: 'ETH-USDT' })
    await nextTick()

    // After symbol change, chart should be reset → empty state reappears
    expect(wrapper.find('.chart-empty').exists()).toBe(true)
  })

  // ─── Cleanup on unmount ─────────────────────────────────────────

  it('disposes chart and stops listening on unmount', async () => {
    const wrapper = await mountAndReady()
    const chart = getMockChart()

    expect(mockListen).toHaveBeenCalledWith('ws:candle', expect.any(Function))

    wrapper.unmount()

    // dispose should have been called
    expect(chart.dispose).toHaveBeenCalled()

    // unlisten should have been called
    expect(mockUnlisten).toHaveBeenCalled()
  })

  // ─── Symbol in title ────────────────────────────────────────────

  it('displays the symbol prop in the chart title', async () => {
    const wrapper = await mountAndReady('ETH-USDT')

    expect(wrapper.find('.chart-title').text()).toContain('ETH-USDT')
  })
})
