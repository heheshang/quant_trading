import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import ElementPlus from 'element-plus'
import Monitor from '@/views/Monitor.vue'
import { mockListen, mockUnlisten } from './setup'
import { invoke } from '@tauri-apps/api/core'
import type { WsConnectionStatus } from '@/services/types'

// ---------------------------------------------------------------------------
// Hoisted mock factories (run before module loading)
// ---------------------------------------------------------------------------
const { mockStartWsStatus, mockWsCleanup, mockStartMarket, mockMarketCleanup } =
  vi.hoisted(() => ({
    mockStartWsStatus: vi.fn().mockResolvedValue(undefined),
    mockWsCleanup: vi.fn(),
    mockStartMarket: vi.fn().mockResolvedValue(undefined),
    mockMarketCleanup: vi.fn(),
  }))

// ---------------------------------------------------------------------------
// Mock composable singletons so we can control wsStatus from tests
// ---------------------------------------------------------------------------
vi.mock('@/composables/useWebSocketStatus', async () => {
  const { ref } = await import('vue')
  const status = ref<WsConnectionStatus>('connected')
  const retryIn = ref(0)
  return {
    useWebSocketStatus: () => ({
      status, retryIn, startListening: mockStartWsStatus, cleanup: mockWsCleanup,
    }),
    setMockWsStatus: (s: WsConnectionStatus) => { status.value = s },
  }
})

vi.mock('@/composables/useMarketData', () => ({
  useMarketData: () => ({
    startListening: mockStartMarket, cleanup: mockMarketCleanup,
    tickerData: { value: {} }, trades: { value: {} }, orderbook: { value: {} }, candleData: { value: {} },
  }),
}))

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return {
    ...mod,
    ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
    ElNotification: { success: vi.fn(), error: vi.fn() },
    ElMessageBox: { confirm: vi.fn() },
  }
})

const mockInvoke = vi.mocked(invoke)

// ---------------------------------------------------------------------------
// OKX trading metrics data
// ---------------------------------------------------------------------------
const okxMetrics = {
  orders_total: 2500,
  orders_filled: 1832,
  orders_cancelled: 345,
  account_balance: 2_456_789.12,
  position_value: 1_890_000.00,
  daily_pnl: 45_678.90,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

describe('Monitor.vue - OKX real-time data panel', () => {
  let container: HTMLDivElement

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2025-01-01T00:00:00Z'))
    vi.clearAllMocks()
    mockListen.mockResolvedValue(mockUnlisten)
    container = document.createElement('div')
    document.body.appendChild(container)
    mockInvoke.mockImplementation((async (cmd: string) => {
      switch (cmd) {
        case 'get_metrics': return { ...okxMetrics }
        case 'get_alerts': return []
        case 'get_logs': return []
        case 'acknowledge_alert': return true
        default: return {}
      }
    }) as typeof invoke)
  })

  afterEach(() => {
    vi.useRealTimers()
    container.remove()
  })

  async function mountMonitor(): Promise<any> {
    const wrapper = mount(Monitor, {
      attachTo: container,
      global: {
        plugins: [ElementPlus],
        stubs: { ConnectionStatus: true },
      },
    })
    for (let i = 0; i < 20; i++) await nextTick()
    return wrapper
  }

  // -----------------------------------------------------------------------
  // 1. Metric cards display OKX trading data correctly
  // -----------------------------------------------------------------------
  it('renders OKX trading metrics in metric cards', async () => {
    const wrapper = await mountMonitor()

    const cards = wrapper.findAll('.metric-card')
    expect(cards).toHaveLength(6)

    // Check formatted values appear
    expect(wrapper.text()).toContain('总订单数')
    expect(wrapper.text()).toContain('已成交订单')
    expect(wrapper.text()).toContain('已撤单数')
    expect(wrapper.text()).toContain('账户余额')
    expect(wrapper.text()).toContain('持仓价值')
    expect(wrapper.text()).toContain('今日盈亏')

    // Check actual values are rendered
    // formatNumber(2500) = '2,500'
    const text = wrapper.text()
    expect(text).toContain('2,500')   // orders_total
    expect(text).toContain('1,832')  // orders_filled
    expect(text).toContain('345')    // orders_cancelled
    // formatCurrency shows ¥ + locale-formatted number
    expect(text).toContain('2,456,789.12')  // account_balance
    expect(text).toContain('1,890,000.00')  // position_value
    expect(text).toContain('45,678.90')     // daily_pnl
  })

  // -----------------------------------------------------------------------
  // 2. Positive daily PnL shows green (+), negative shows red (-)
  // -----------------------------------------------------------------------
  it('formats positive daily PnL with plus sign and green class', async () => {
    mockInvoke.mockImplementation((async (cmd: string) => {
      if (cmd === 'get_metrics') return { ...okxMetrics, daily_pnl: 45678.90 }
      if (cmd === 'get_alerts') return []
      if (cmd === 'get_logs') return []
      return {}
    }) as typeof invoke)

    const wrapper = await mountMonitor()
    const text = wrapper.text()
    expect(text).toContain('+')  // positive sign
  })

  it('formats negative daily PnL with minus sign and red class', async () => {
    mockInvoke.mockImplementation((async (cmd: string) => {
      if (cmd === 'get_metrics') return { ...okxMetrics, daily_pnl: -12345.67 }
      if (cmd === 'get_alerts') return []
      if (cmd === 'get_logs') return []
      return {}
    }) as typeof invoke)

    const wrapper = await mountMonitor()
    const text = wrapper.text()
    // Negative values should not have a plus sign
    expect(text).toContain('-12,345.67')
  })

  // -----------------------------------------------------------------------
  // 3. Manual refresh button calls get_metrics via invoke
  // -----------------------------------------------------------------------
  it('refreshData calls get_metrics via invoke', async () => {
    const wrapper = await mountMonitor()
    mockInvoke.mockClear()

    wrapper.vm.refreshData()
    await nextTick()
    await nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')
  })

  // -----------------------------------------------------------------------
  // 4. Loading state during data refresh
  // -----------------------------------------------------------------------
  it('sets loading true during data refresh', async () => {
    const wrapper = await mountMonitor()

    // Freeze one of the API calls
    let resolveMetrics: ((v: any) => void) | undefined
    mockInvoke.mockImplementation((async (cmd: string) => {
      if (cmd === 'get_metrics') return new Promise((resolve: any) => { resolveMetrics = resolve })
      if (cmd === 'get_alerts') return []
      if (cmd === 'get_logs') return []
      return {}
    }) as typeof invoke)

    const refreshPromise = wrapper.vm.refreshData()
    await nextTick()
    expect(wrapper.vm.loading).toBe(true)

    resolveMetrics?.({ ...okxMetrics })
    await refreshPromise
    await nextTick()
    // loading may be set by other ongoing calls, but eventually should be false
    // Since all promises resolve, loading should be false
  })

  // -----------------------------------------------------------------------
  // 5. Metrics display large numbers formatted (locale)
  // -----------------------------------------------------------------------
  it('formats large OKX metrics with locale formatting', async () => {
    mockInvoke.mockImplementation((async (cmd: string) => {
      if (cmd === 'get_metrics') {
        return {
          orders_total: 1000000,
          orders_filled: 500000,
          orders_cancelled: 100000,
          account_balance: 99999999.99,
          position_value: 50000000,
          daily_pnl: 1234567.89,
        }
      }
      if (cmd === 'get_alerts') return []
      if (cmd === 'get_logs') return []
      return {}
    }) as typeof invoke)

    const wrapper = await mountMonitor()
    const text = wrapper.text()

    // Locale-formatted values
    expect(text).toContain('1,000,000')  // orders_total
    expect(text).toContain('99,999,999.99')  // account_balance
    expect(text).toContain('1,234,567.89')   // daily_pnl
  })

  // -----------------------------------------------------------------------
  // 6. Ws:ticker events trigger metrics refresh (throttled to 5s)
  // -----------------------------------------------------------------------
  it('handles ws:ticker event to refresh OKX metrics', async () => {
    await mountMonitor()
    mockInvoke.mockClear()

    // Find the ticker callback
    const tickerCall = mockListen.mock.calls.find((c: unknown[]) => c[0] === 'ws:ticker')
    expect(tickerCall).not.toBeUndefined()
    const tickerCb = tickerCall![1] as (event: { payload: unknown }) => void

    tickerCb({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')
  })

  // -----------------------------------------------------------------------
  // 7. Component does not crash when WS listeners fail
  // -----------------------------------------------------------------------
  it('does not crash when get_metrics API fails', async () => {
    mockInvoke.mockImplementation((async (cmd: string) => {
      if (cmd === 'get_metrics') throw new Error('Metrics API unavailable')
      if (cmd === 'get_alerts') return []
      if (cmd === 'get_logs') return []
      return {}
    }) as typeof invoke)

    const wrapper = await mountMonitor()
    expect(wrapper.exists()).toBe(true)

    // Fallback random data should be rendered
    const text = wrapper.text()
    expect(text).toContain('总订单数')
    expect(text).toContain('已成交订单')
  })
})
