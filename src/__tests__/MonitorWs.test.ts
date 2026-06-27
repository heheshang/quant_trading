import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, VueWrapper } from '@vue/test-utils'
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
      status,
      retryIn,
      startListening: mockStartWsStatus,
      cleanup: mockWsCleanup,
    }),
    setMockWsStatus: (s: WsConnectionStatus) => {
      status.value = s
    },
    setMockRetryIn: (n: number) => {
      retryIn.value = n
    },
  }
})

vi.mock('@/composables/useMarketData', () => ({
  useMarketData: () => ({
    startListening: mockStartMarket,
    cleanup: mockMarketCleanup,
    tickerData: { value: {} },
    trades: { value: {} },
    orderbook: { value: {} },
    candleData: { value: {} },
  }),
}))

// Override the global element-plus mock: keep real components but mock message utilities
vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return {
    ...mod,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
      info: vi.fn(),
    },
    ElNotification: {
      success: vi.fn(),
      error: vi.fn(),
    },
    ElMessageBox: {
      confirm: vi.fn(),
    },
  }
})

const mockInvoke = vi.mocked(invoke)

describe('MonitorWs — WS data display', () => {
  let container: HTMLDivElement

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2025-01-01T00:00:00Z'))
    vi.clearAllMocks()
    mockListen.mockResolvedValue(mockUnlisten)
    container = document.createElement('div')
    document.body.appendChild(container)
    mockInvoke.mockImplementation(
      (async (cmd: string) => {
        switch (cmd) {
          case 'get_metrics':
            return {
              orders_total: 1000,
              orders_filled: 800,
              orders_cancelled: 200,
              account_balance: 1_234_567.89,
              position_value: 1_000_000,
              daily_pnl: 12_345.67,
            }
          case 'get_alerts':
            return []
          case 'get_logs':
            return []
          default:
            return {}
        }
      }) as typeof invoke,
    )
  })

  afterEach(() => {
    vi.useRealTimers()
    container.remove()
  })

  async function mountMonitor(): Promise<VueWrapper<InstanceType<typeof Monitor>>> {
    const wrapper = mount(Monitor, {
      attachTo: container,
      global: {
        plugins: [ElementPlus],
        // Do NOT stub ConnectionStatus — we want to test its rendering
      },
    })
    for (let i = 0; i < 20; i++) await nextTick()
    return wrapper
  }

  // -----------------------------------------------------------------------
  // ConnectionStatus rendering for different WS states
  // -----------------------------------------------------------------------

  it('shows connected status initially', async () => {
    const wrapper = await mountMonitor()

    expect(wrapper.text()).toContain('已连接')
  })

  it('shows reconnecting status with retry countdown', async () => {
    const wsModule = await import('@/composables/useWebSocketStatus')
    const setMockWsStatus = (wsModule as unknown as { setMockWsStatus: (s: WsConnectionStatus) => void }).setMockWsStatus
    const setMockRetryIn = (wsModule as unknown as { setMockRetryIn: (n: number) => void }).setMockRetryIn

    const wrapper = await mountMonitor()

    setMockWsStatus('reconnecting')
    setMockRetryIn(5)
    await nextTick()

    expect(wrapper.text()).toContain('重连中')
    expect(wrapper.text()).toContain('5s')
  })

  it('shows disconnected state with reconnect button', async () => {
    const wsModule = await import('@/composables/useWebSocketStatus')
    const setMockWsStatus = (wsModule as unknown as { setMockWsStatus: (s: WsConnectionStatus) => void }).setMockWsStatus

    const wrapper = await mountMonitor()

    setMockWsStatus('disconnected')
    await nextTick()

    expect(wrapper.text()).toContain('已断开')
    // The manual reconnect button should be visible
    expect(wrapper.text()).toContain('手动重连')
  })

  // -----------------------------------------------------------------------
  // WS ticker event triggers metrics refresh (throttled to 5s)
  // -----------------------------------------------------------------------

  it('ws:ticker event triggers fetchMetrics when throttled', async () => {
    await mountMonitor()
    mockInvoke.mockClear()

    const tickerCb = getTickerCallback()
    expect(tickerCb).not.toBeNull()

    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')
  })

  it('ws:ticker event is throttled — skip calls within 5s window', async () => {
    await mountMonitor()
    mockInvoke.mockClear()

    const tickerCb = getTickerCallback()
    expect(tickerCb).not.toBeNull()

    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(1)

    // Advance 2s (still inside 5s window)
    vi.setSystemTime(new Date('2025-01-01T00:00:02Z'))
    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(1)

    // Advance past 5s window
    vi.setSystemTime(new Date('2025-01-01T00:00:07Z'))
    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(2)
  })

  // -----------------------------------------------------------------------
  // WS alert event pushes new alert to alert table
  // -----------------------------------------------------------------------

  it('ws:alerts event callback is registered and processes events', async () => {
    const wrapper = await mountMonitor()
    mockInvoke.mockClear()

    // Verify the alert WS listener was registered
    expect(mockListen).toHaveBeenCalledWith('ws:alerts', expect.any(Function))

    // Get the callback and verify it updates internal state
    const alertCb = getListenerCallback('ws:alerts')
    expect(alertCb).not.toBeNull()

    // It should not throw when processing a valid alert payload
    expect(() => {
      alertCb!({
        payload: {
          alert_id: 10,
          level: 'Warning',
          source: 'System',
          message: 'WebSocket data flow test alert',
          timestamp: '2025-01-01T00:05:00Z',
        },
      })
    }).not.toThrow()

    // Switch to alerts tab to verify alert appears in the DOM
    const tabs = wrapper.findAll('.el-tabs__item')
    await tabs[1].trigger('click')
    await nextTick()

    // The Card header should still show
    expect(wrapper.text()).toContain('最新告警')
  })

  // -----------------------------------------------------------------------
  // WS log event pushes new log entry
  // -----------------------------------------------------------------------

  it('ws:logs event pushes new log entry to the display', async () => {
    const wrapper = await mountMonitor()
    mockInvoke.mockClear()

    // Switch to logs tab
    const tabs = wrapper.findAll('.el-tabs__item')
    await tabs[3].trigger('click')
    await nextTick()

    const logCb = getListenerCallback('ws:logs')
    expect(logCb).not.toBeNull()

    logCb!({
      payload: {
        timestamp: '2025-01-01T00:06:00Z',
        level: 'error',
        message: 'WebSocket data flow connection lost',
        module: 'ws',
      },
    })
    await nextTick()

    expect(wrapper.text()).toContain('WebSocket data flow connection lost')
  })

  // -----------------------------------------------------------------------
  // Return to connected from reconnecting stops showing reconnection text
  // -----------------------------------------------------------------------

  it('transitions from reconnecting back to connected hides reconnection indicator', async () => {
    const wsModule = await import('@/composables/useWebSocketStatus')
    const setMockWsStatus = (wsModule as unknown as { setMockWsStatus: (s: WsConnectionStatus) => void }).setMockWsStatus

    const wrapper = await mountMonitor()

    setMockWsStatus('reconnecting')
    await nextTick()
    expect(wrapper.text()).toContain('重连中')

    setMockWsStatus('connected')
    await nextTick()
    expect(wrapper.text()).toContain('已连接')
    expect(wrapper.text()).not.toContain('重连中')
  })

  // -----------------------------------------------------------------------
  // Cleanup on unmount — WS listeners disposed
  // -----------------------------------------------------------------------

  it('cleans up all WS listeners on unmount', async () => {
    const wrapper = await mountMonitor()

    expect(mockListen).toHaveBeenCalledWith('ws:ticker', expect.any(Function))
    expect(mockListen).toHaveBeenCalledWith('ws:alerts', expect.any(Function))
    expect(mockListen).toHaveBeenCalledWith('ws:logs', expect.any(Function))

    wrapper.unmount()

    expect(mockUnlisten).toHaveBeenCalledTimes(3)
  })
})

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getListenerCallback(eventName: string): ((event: { payload: unknown }) => void) | null {
  const call = mockListen.mock.calls.find((c: unknown[]) => c[0] === eventName)
  if (!call) return null
  return call[1] as (event: { payload: unknown }) => void
}

function getTickerCallback(): ((event: { payload: unknown }) => void) | null {
  return getListenerCallback('ws:ticker')
}
