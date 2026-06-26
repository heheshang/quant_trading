import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import ElementPlus from 'element-plus'
import Monitor from '@/views/Monitor.vue'
import { mockListen, mockUnlisten } from './setup'
import { invoke } from '@tauri-apps/api/core'
import * as echartsModule from 'echarts'
import type { Alert, LogEntry, WsConnectionStatus } from '@/services/types'

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

// ---------------------------------------------------------------------------
// Mock Element Plus: use real components, keep ElMessage etc. mocked
// ---------------------------------------------------------------------------
vi.mock('element-plus', async () => {
  const mod =
    await vi.importActual<typeof import('element-plus')>('element-plus')
  return {
    ...mod,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
      info: vi.fn(),
    },
    ElNotification: { success: vi.fn(), error: vi.fn() },
    ElMessageBox: { confirm: vi.fn() },
  }
})

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
const mockInvoke = vi.mocked(invoke)

/** Retrieve the listener callback registered for a given WS event name. */
function getListenerCallback(
  eventName: string,
): ((event: { payload: unknown }) => void) | null {
  const call = mockListen.mock.calls.find(
    (c: unknown[]) => c[0] === eventName,
  )
  if (!call) return null
  return call[1] as (event: { payload: unknown }) => void
}

// Get the shared mock echarts instance (init always returns the same mock)
function getChartMock() {
  return echartsModule.init(document.createElement('div'))
}

// Default data returned by mocked invoke
const defaultMetrics = {
  orders_total: 1000,
  orders_filled: 800,
  orders_cancelled: 200,
  account_balance: 1_234_567.89,
  position_value: 1_000_000,
  daily_pnl: 12_345.67,
}

const defaultAlerts: Alert[] = [
  {
    alert_id: 1,
    level: 'Warning',
    source: 'Risk Management',
    message: 'Margin ratio approaching limit',
    timestamp: '2025-01-01T00:00:00Z',
    acknowledged: false,
  },
  {
    alert_id: 2,
    level: 'Critical',
    source: 'Trading Engine',
    message: 'Order latency exceeded threshold',
    timestamp: '2025-01-01T00:05:00Z',
    acknowledged: false,
  },
]

const defaultLogs: LogEntry[] = [
  {
    timestamp: '2025-01-01T00:00:00Z',
    level: 'info',
    message: 'System started successfully',
    module: 'system',
  },
  {
    timestamp: '2025-01-01T00:01:00Z',
    level: 'warning',
    message: 'Margin ratio limit',
    module: 'risk',
  },
]

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------
describe('Monitor', () => {
  let container: HTMLDivElement

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2025-01-01T00:00:00Z'))

    vi.clearAllMocks()
    mockListen.mockResolvedValue(mockUnlisten)

    container = document.createElement('div')
    document.body.appendChild(container)

    // Default invoke behaviour: return canned data per command
    mockInvoke.mockImplementation((async (cmd: string) => {
      switch (cmd) {
        case 'get_metrics':
          return { ...defaultMetrics }
        case 'get_alerts':
          return [...defaultAlerts]
        case 'get_logs':
          return [...defaultLogs]
        case 'acknowledge_alert':
          return true
        default:
          return {}
      }
    }) as typeof invoke)

    mockListen.mockResolvedValue(mockUnlisten)
  })

  afterEach(() => {
    vi.useRealTimers()
    container.remove()
  })

  /** Mount Monitor and wait for onMounted async flow to settle. */
  async function mountMonitor(): Promise<VueWrapper<InstanceType<typeof Monitor>>> {
    const wrapper = mount(Monitor, {
      attachTo: container,
      global: {
        plugins: [ElementPlus],
        stubs: {
          // Stub child component — we test Monitor, not ConnectionStatus
          ConnectionStatus: true,
        },
      },
    })
    // Flush the async onMounted chain: initMetricsChart → startWsStatusListening →
    // refreshData (3 invoke calls) → startWsListeners (3 listen calls)
    for (let i = 0; i < 20; i++) {
      await nextTick()
    }
    return wrapper
  }

  // -----------------------------------------------------------------------
  // 1. Component mounts and renders metric cards + tabs
  // -----------------------------------------------------------------------
  it('renders 6 metric cards and 3 tabs on mount', async () => {
    const wrapper = await mountMonitor()

    // Tabs
    const tabs = wrapper.findAll('.el-tabs__item')
    expect(tabs).toHaveLength(3)
    expect(tabs[0].text()).toBe('指标监控')
    expect(tabs[1].text()).toBe('告警监控')
    expect(tabs[2].text()).toBe('系统日志')

    // Metric cards (6 cards in 2 rows)
    const cards = wrapper.findAll('.metric-card')
    expect(cards).toHaveLength(6)

    // Verify metric values rendered
    expect(wrapper.text()).toContain('总订单数')
    expect(wrapper.text()).toContain('已成交订单')
    expect(wrapper.text()).toContain('已撤单数')
    expect(wrapper.text()).toContain('账户余额')
    expect(wrapper.text()).toContain('持仓价值')
    expect(wrapper.text()).toContain('今日盈亏')
  })

  // -----------------------------------------------------------------------
  // 2. WS ticker event — throttle (5 s)
  // -----------------------------------------------------------------------
  it('throttles ws:ticker events to one fetchMetrics per 5 s', async () => {
    await mountMonitor()

    // onMounted refreshData already called fetchMetrics once via invoke
    // Clear history so we measure only ticker-triggered calls
    mockInvoke.mockClear()

    const tickerCb = getListenerCallback('ws:ticker')
    expect(tickerCb).not.toBeNull()

    // First ticker event → should trigger fetchMetrics
    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')

    // Advance 2 s — second ticker event (within 5 s window → throttled)
    vi.setSystemTime(new Date('2025-01-01T00:00:02Z'))
    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(1) // still 1

    // Advance to 7 s — third ticker event (outside 5 s window → fires)
    vi.setSystemTime(new Date('2025-01-01T00:00:07Z'))
    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(2)
  })

  // -----------------------------------------------------------------------
  // 3. WS alert event pushes new alert
  // -----------------------------------------------------------------------
  it('ws:alerts event pushes a new alert to the alerts table', async () => {
    const wrapper = await mountMonitor()

    // Switch to alerts tab so the table is visible
    const alertsTab = wrapper.findAll('.el-tabs__item')[1]
    await alertsTab.trigger('click')
    await nextTick()

    const alertCb = getListenerCallback('ws:alerts')
    expect(alertCb).not.toBeNull()

    alertCb!({
      payload: {
        alert_id: 'a3',
        level: 'Critical',
        source: 'Trading Engine',
        message: 'New critical alert from WS',
        timestamp: '2025-01-01T00:02:00Z',
      },
    })
    await nextTick()

    // Should now have 3 alerts (2 default + 1 pushed)
    const rows = wrapper.findAll('.el-table__body tbody tr')
    expect(rows).toHaveLength(3)
    // The newest alert is prepended
    expect(rows[0].text()).toContain('New critical alert from WS')
  })

  // -----------------------------------------------------------------------
  // 4. WS log event pushes new log entry
  // -----------------------------------------------------------------------
  it('ws:logs event pushes a new log entry', async () => {
    const wrapper = await mountMonitor()

    // Switch to logs tab
    const logsTab = wrapper.findAll('.el-tabs__item')[2]
    await logsTab.trigger('click')
    await nextTick()

    const logCb = getListenerCallback('ws:logs')
    expect(logCb).not.toBeNull()

    logCb!({
      payload: {
        timestamp: '2025-01-01T00:03:00Z',
        level: 'error',
        message: 'Connection timeout',
        module: 'network',
      },
    })
    await nextTick()

    const entries = wrapper.findAll('.log-entry')
    // 2 default + 1 pushed = 3
    expect(entries).toHaveLength(3)
    expect(entries[0].text()).toContain('Connection timeout')
    expect(entries[0].classes()).toContain('log-error')
  })

  // -----------------------------------------------------------------------
  // 5. Alert acknowledgment
  // -----------------------------------------------------------------------
  it('acknowledgeAlert calls invoke and updates acknowledged flag', async () => {
    const wrapper = await mountMonitor()

    // Switch to alerts tab
    const alertsTab = wrapper.findAll('.el-tabs__item')[1]
    await alertsTab.trigger('click')
    await nextTick()

    // First alert row
    const firstRow = wrapper.find('.el-table__body tbody tr')
    const ackBtn = firstRow.find('.el-button')
    expect(ackBtn.text()).toBe('确认')
    expect(ackBtn.attributes('disabled')).toBeUndefined()

    // Click acknowledge
    await ackBtn.trigger('click')
    await nextTick()
    await nextTick() // extra tick for invoke + reactivity

    // Verify invoke was called
    expect(mockInvoke).toHaveBeenCalledWith('acknowledge_alert', {
      alertId: 1,
    })

    // Button should now show "已确认" and be disabled
    expect(ackBtn.text()).toBe('已确认')
    expect(ackBtn.attributes('disabled')).toBeDefined()
  })

  // -----------------------------------------------------------------------
  // 6. Tab switching
  // -----------------------------------------------------------------------
  it('switches between metrics, alerts, and logs tabs', async () => {
    const wrapper = await mountMonitor()

    const tabs = wrapper.findAll('.el-tabs__item')

    // Default: metrics tab active
    // Use isVisible() because Element Plus tabs keep all panes in DOM
    // with v-show (display: none for inactive)
    expect(wrapper.find('#metrics-chart').exists()).toBe(true)
    expect(wrapper.find('.el-table').isVisible()).toBe(false)
    expect(wrapper.find('.log-container').isVisible()).toBe(false)

    // Switch to alerts
    await tabs[1].trigger('click')
    await nextTick()
    expect(wrapper.find('#metrics-chart').isVisible()).toBe(false)
    expect(wrapper.find('.el-table').isVisible()).toBe(true)
    expect(wrapper.find('.log-container').isVisible()).toBe(false)

    // Switch to logs
    await tabs[2].trigger('click')
    await nextTick()
    expect(wrapper.find('#metrics-chart').isVisible()).toBe(false)
    expect(wrapper.find('.el-table').isVisible()).toBe(false)
    expect(wrapper.find('.log-container').isVisible()).toBe(true)

    // Switch back to metrics
    await tabs[0].trigger('click')
    await nextTick()
    expect(wrapper.find('#metrics-chart').exists()).toBe(true)
    expect(wrapper.find('.el-table').isVisible()).toBe(false)
    expect(wrapper.find('.log-container').isVisible()).toBe(false)
  })

  // -----------------------------------------------------------------------
  // 7. Polling fallback triggers 60 s after disconnect
  // -----------------------------------------------------------------------
  it('starts polling fallback 60 s after disconnect', async () => {
    // Import the mock setter to trigger disconnect
    const wsModule = await import('@/composables/useWebSocketStatus')
    const setMockWsStatus = (
      wsModule as unknown as {
        setMockWsStatus: (s: WsConnectionStatus) => void
      }
    ).setMockWsStatus

    const wrapper = await mountMonitor()

    // onMounted refreshData already called invoke — clear for clean measurement
    mockInvoke.mockClear()

    // Trigger disconnect
    setMockWsStatus('disconnected')
    await nextTick()

    // Polling badge should NOT appear yet
    expect(wrapper.find('.polling-badge').exists()).toBe(false)

    // Advance 61 s to fire the 60 s disconnect timer
    await vi.advanceTimersByTimeAsync(61_000)

    // Polling badge should now appear
    expect(wrapper.find('.polling-badge').exists()).toBe(true)
    expect(wrapper.find('.polling-badge').text()).toBe('轮询模式')

    // Advance 6 s to fire the first 5 s polling interval
    await vi.advanceTimersByTimeAsync(6_000)

    // First poll should have called all three fetch commands
    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')
    expect(mockInvoke).toHaveBeenCalledWith('get_alerts')
    expect(mockInvoke).toHaveBeenCalledWith('get_logs', expect.objectContaining({ level: null, limit: 50 }))
  })

  // -----------------------------------------------------------------------
  // 8. onUnmounted cleanup
  // -----------------------------------------------------------------------
  it('cleans up WS listeners and disposes ECharts on unmount', async () => {
    const wrapper = await mountMonitor()

    // Verify listeners were registered
    expect(mockListen).toHaveBeenCalledWith('ws:ticker', expect.any(Function))
    expect(mockListen).toHaveBeenCalledWith('ws:alerts', expect.any(Function))
    expect(mockListen).toHaveBeenCalledWith('ws:logs', expect.any(Function))

    wrapper.unmount()

    // Each listener returns mockUnlisten; all 3 should have been called
    expect(mockUnlisten).toHaveBeenCalledTimes(3)

    // ECharts instance should be disposed
    expect(getChartMock().dispose).toHaveBeenCalled()
  })

  it('clears disconnect timer on unmount so polling never starts', async () => {
    const wsModule = await import('@/composables/useWebSocketStatus')
    const setMockWsStatus = (
      wsModule as unknown as {
        setMockWsStatus: (s: WsConnectionStatus) => void
      }
    ).setMockWsStatus

    const wrapper = await mountMonitor()
    mockInvoke.mockClear()

    // Trigger disconnect
    setMockWsStatus('disconnected')
    await nextTick()

    // Unmount BEFORE the 60 s timer fires
    wrapper.unmount()

    // Advance well past 60 s
    await vi.advanceTimersByTimeAsync(120_000)

    // Polling should never have started — no invoke calls
    expect(mockInvoke).not.toHaveBeenCalled()
  })
})
