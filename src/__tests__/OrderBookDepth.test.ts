import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import * as echarts from 'echarts'
import OrderBookDepth from '../components/dashboard/OrderBookDepth.vue'
import { mockListen, mockUnlisten } from './setup'
import type { WsOrderBook } from '../services/types'

function makeOrderBook(overrides: Partial<WsOrderBook> = {}): WsOrderBook {
  return {
    inst_id: 'BTC-USDT',
    asks: [
      ['50000', '1.5'],
      ['50100', '2.0'],
    ],
    bids: [
      ['49900', '1.0'],
      ['49800', '3.0'],
    ],
    ts: '1234567890',
    ...overrides,
  }
}

/**
 * echarts.init is mocked via vi.mock in setup.ts and always returns the same
 * mock ECharts instance. Use this helper to retrieve it for assertions.
 */
function getChartMock(): {
  setOption: ReturnType<typeof vi.fn>
  dispose: ReturnType<typeof vi.fn>
} {
  const mockedInit = echarts.init as unknown as ReturnType<typeof vi.fn>
  return mockedInit.mock.results[0]?.value as {
    setOption: ReturnType<typeof vi.fn>
    dispose: ReturnType<typeof vi.fn>
  }
}

describe('OrderBookDepth', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('mounts and renders chart container div with ref="chartRef"', () => {
    const wrapper = mount(OrderBookDepth, {
      props: { symbol: 'BTC-USDT' },
    })

    const chartDiv = wrapper.find('.chart-container')
    expect(chartDiv.exists()).toBe(true)
    expect(chartDiv.element.tagName).toBe('DIV')
  })

  it('calls echarts.init on mount with the chart container element', () => {
    mount(OrderBookDepth, {
      props: { symbol: 'BTC-USDT' },
    })

    expect(echarts.init).toHaveBeenCalledTimes(1)
    expect(echarts.init).toHaveBeenCalledWith(expect.any(HTMLDivElement))
  })

  it('calls echarts.setOption when receiving orderbook event with matching symbol', async () => {
    mount(OrderBookDepth, {
      props: { symbol: 'BTC-USDT' },
    })
    await nextTick()

    expect(mockListen).toHaveBeenCalledWith(
      'ws:orderbook',
      expect.any(Function),
    )

    const callback = mockListen.mock.calls[0][1] as (
      event: { payload: WsOrderBook },
    ) => void

    callback({ payload: makeOrderBook({ inst_id: 'BTC-USDT' }) })

    const chart = getChartMock()
    expect(chart.setOption).toHaveBeenCalled()
  })

  it('ignores orderbook event when inst_id does not match symbol prop', async () => {
    mount(OrderBookDepth, {
      props: { symbol: 'BTC-USDT' },
    })
    await nextTick()

    const callback = mockListen.mock.calls[0][1] as (
      event: { payload: WsOrderBook },
    ) => void

    callback({ payload: makeOrderBook({ inst_id: 'ETH-USDT' }) })

    const chart = getChartMock()
    expect(chart.setOption).not.toHaveBeenCalled()
  })

  it('throttles rapid updates: only first within 500ms triggers setOption', async () => {
    vi.useFakeTimers()

    mount(OrderBookDepth, {
      props: { symbol: 'BTC-USDT' },
    })

    // Flush microtasks so setupListener's async await completes
    vi.runAllTicks()

    const callback = mockListen.mock.calls[0][1] as (
      event: { payload: WsOrderBook },
    ) => void

    // Advance time so first event passes the leading-edge throttle
    vi.advanceTimersByTime(1000)

    // First event: should trigger setOption
    callback({ payload: makeOrderBook({ inst_id: 'BTC-USDT' }) })
    const chart = getChartMock()
    expect(chart.setOption).toHaveBeenCalledTimes(1)

    chart.setOption.mockClear()

    // Second event immediately after: within 500ms → throttled
    callback({
      payload: makeOrderBook({ inst_id: 'BTC-USDT', ts: '2' }),
    })
    expect(chart.setOption).not.toHaveBeenCalled()

    // Advance past the 500ms throttle window
    vi.advanceTimersByTime(600)

    // Third event: should trigger setOption again
    callback({
      payload: makeOrderBook({ inst_id: 'BTC-USDT', ts: '3' }),
    })
    expect(chart.setOption).toHaveBeenCalledTimes(1)

    vi.useRealTimers()
  })

  it('calls chartInstance.dispose and unlistenFn on unmount', async () => {
    const wrapper = mount(OrderBookDepth, {
      props: { symbol: 'BTC-USDT' },
    })
    await nextTick()

    const chart = getChartMock()

    wrapper.unmount()

    expect(chart.dispose).toHaveBeenCalled()
    expect(mockUnlisten).toHaveBeenCalled()
  })
})
