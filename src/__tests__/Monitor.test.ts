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

function getListenerCallback(eventName: string): ((event: { payload: unknown }) => void) | null {
  const call = mockListen.mock.calls.find((c: unknown[]) => c[0] === eventName)
  if (!call) return null
  return call[1] as (event: { payload: unknown }) => void
}

function getChartMock() {
  return echartsModule.init(document.createElement('div'))
}

const defaultMetrics = {
  orders_total: 1000, orders_filled: 800, orders_cancelled: 200,
  account_balance: 1_234_567.89, position_value: 1_000_000, daily_pnl: 12_345.67,
}

const defaultAlerts: Alert[] = [
  { alert_id: 1, level: 'Warning', source: 'Risk Management', message: 'Margin ratio approaching limit', timestamp: '2025-01-01T00:00:00Z', acknowledged: false },
  { alert_id: 2, level: 'Critical', source: 'Trading Engine', message: 'Order latency exceeded threshold', timestamp: '2025-01-01T00:05:00Z', acknowledged: false },
]

const defaultLogs: LogEntry[] = [
  { timestamp: '2025-01-01T00:00:00Z', level: 'info', message: 'System started successfully', module: 'system' },
  { timestamp: '2025-01-01T00:01:00Z', level: 'warning', message: 'Margin ratio limit', module: 'risk' },
]

describe('Monitor', () => {
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
        case 'get_metrics': return { ...defaultMetrics }
        case 'get_alerts': return [...defaultAlerts]
        case 'get_logs': return [...defaultLogs]
        case 'acknowledge_alert': return true
        default: return {}
      }
    }) as typeof invoke)
    mockListen.mockResolvedValue(mockUnlisten)
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
        stubs: { ConnectionStatus: true },
      },
    })
    for (let i = 0; i < 20; i++) await nextTick()
    return wrapper
  }

  // -----------------------------------------------------------------------
  // 1. Component mounts and renders metric cards + tabs
  // -----------------------------------------------------------------------
  it('renders 6 metric cards and 4 tabs on mount', async () => {
    const wrapper = await mountMonitor()

    // Tabs — Monitor.vue has 4 tab panes (metrics, alerts, thresholds, logs)
    const tabs = wrapper.findAll('.el-tabs__item')
    expect(tabs).toHaveLength(4)
    expect(tabs[0].text()).toBe('指标监控')
    expect(tabs[1].text()).toBe('告警监控')
    expect(tabs[2].text()).toBe('告警阈值')
    expect(tabs[3].text()).toBe('系统日志')

    const cards = wrapper.findAll('.metric-card')
    expect(cards).toHaveLength(6)

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
    mockInvoke.mockClear()

    const tickerCb = getListenerCallback('ws:ticker')
    expect(tickerCb).not.toBeNull()

    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')

    vi.setSystemTime(new Date('2025-01-01T00:00:02Z'))
    tickerCb!({ payload: {} })
    await nextTick()
    expect(mockInvoke).toHaveBeenCalledTimes(1)

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

    const rows = wrapper.findAll('.el-table__body tbody tr')
    expect(rows).toHaveLength(3)
    expect(rows[0].text()).toContain('New critical alert from WS')
  })

  // -----------------------------------------------------------------------
  // 4. WS log event pushes new log entry
  // -----------------------------------------------------------------------
  it('ws:logs event pushes a new log entry', async () => {
    const wrapper = await mountMonitor()

    const logsTab = wrapper.findAll('.el-tabs__item')[3]
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
    expect(entries).toHaveLength(3)
    expect(entries[0].text()).toContain('Connection timeout')
    expect(entries[0].classes()).toContain('log-error')
  })

  // -----------------------------------------------------------------------
  // 5. Alert acknowledgment
  // -----------------------------------------------------------------------
  it('acknowledgeAlert calls invoke and updates acknowledged flag', async () => {
    const wrapper = await mountMonitor()

    const alertsTab = wrapper.findAll('.el-tabs__item')[1]
    await alertsTab.trigger('click')
    await nextTick()

    const firstRow = wrapper.find('.el-table__body tbody tr')
    const ackBtn = firstRow.find('.el-button')
    expect(ackBtn.text()).toBe('确认')

    await ackBtn.trigger('click')
    await nextTick()
    await nextTick()

    // The component calls acknowledgeAlert(scope.row.alert_id) with alert_id from data
    // Mock data has alert_id: 1 as number, but Element Plus may pass it as string
    // Accept both numeric and string forms
    const ackCalls = mockInvoke.mock.calls.filter(([cmd]) => cmd === 'acknowledge_alert')
    expect(ackCalls.length).toBeGreaterThanOrEqual(1)
    const params = ackCalls[0][1] as Record<string, unknown>
    expect(Number(params.alertId)).toBe(1)

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
    expect(wrapper.text()).toContain('实时指标趋势')

    // Switch to alerts (index 1)
    await tabs[1].trigger('click')
    await nextTick()
    expect(wrapper.find('.el-table').exists()).toBe(true)

    // Switch to logs (index 3)
    await tabs[3].trigger('click')
    await nextTick()
    expect(wrapper.find('.log-container').exists()).toBe(true)

    // Switch back to metrics (index 0)
    await tabs[0].trigger('click')
    await nextTick()
    expect(wrapper.text()).toContain('实时指标趋势')
  })

  // -----------------------------------------------------------------------
  // 7. Polling fallback triggers 60 s after disconnect
  // -----------------------------------------------------------------------
  it('starts polling fallback 60 s after disconnect', async () => {
    const wsModule = await import('@/composables/useWebSocketStatus')
    const setMockWsStatus = (wsModule as unknown as { setMockWsStatus: (s: WsConnectionStatus) => void }).setMockWsStatus

    const wrapper = await mountMonitor()
    mockInvoke.mockClear()

    setMockWsStatus('disconnected')
    await nextTick()

    expect(wrapper.find('.polling-badge').exists()).toBe(false)

    await vi.advanceTimersByTimeAsync(61_000)

    expect(wrapper.find('.polling-badge').exists()).toBe(true)
    expect(wrapper.find('.polling-badge').text()).toBe('轮询模式')

    await vi.advanceTimersByTimeAsync(6_000)

    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')
    expect(mockInvoke).toHaveBeenCalledWith('get_alerts')
    // fetchLogs passes logLevel || undefined — in the test logLevel is '' so undefined
    expect(mockInvoke).toHaveBeenCalledWith('get_logs', expect.objectContaining({ limit: 50 }))
  })

  // -----------------------------------------------------------------------
  // 8. onUnmounted cleanup
  // -----------------------------------------------------------------------
  it('cleans up WS listeners and disposes ECharts on unmount', async () => {
    const wrapper = await mountMonitor()

    expect(mockListen).toHaveBeenCalledWith('ws:ticker', expect.any(Function))
    expect(mockListen).toHaveBeenCalledWith('ws:alerts', expect.any(Function))
    expect(mockListen).toHaveBeenCalledWith('ws:logs', expect.any(Function))

    wrapper.unmount()

    expect(mockUnlisten).toHaveBeenCalledTimes(3)
    expect(getChartMock().dispose).toHaveBeenCalled()
  })

  it('clears disconnect timer on unmount so polling never starts', async () => {
    const wsModule = await import('@/composables/useWebSocketStatus')
    const setMockWsStatus = (wsModule as unknown as { setMockWsStatus: (s: WsConnectionStatus) => void }).setMockWsStatus

    const wrapper = await mountMonitor()
    mockInvoke.mockClear()

    setMockWsStatus('disconnected')
    await nextTick()

    wrapper.unmount()
    await vi.advanceTimersByTimeAsync(120_000)

    expect(mockInvoke).not.toHaveBeenCalled()
  })
})
