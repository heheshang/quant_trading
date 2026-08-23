import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import ElementPlus from 'element-plus'
import { createPinia, setActivePinia } from 'pinia'
import Monitor from '@/views/Monitor.vue'
import { invoke } from '@tauri-apps/api/core'
import * as echartsModule from 'echarts'
import type { Alert, LogEntry } from '@/services/types'

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
    vi.clearAllMocks()
    setActivePinia(createPinia())
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
  })

  afterEach(() => {
    container.remove()
  })

  async function mountMonitor(): Promise<VueWrapper<InstanceType<typeof Monitor>>> {
    const wrapper = mount(Monitor, {
      attachTo: container,
      global: {
        plugins: [ElementPlus],
      },
    })
    for (let i = 0; i < 20; i++) await nextTick()
    return wrapper
  }

  it('renders 6 metric cards and 6 tabs on mount', async () => {
    const wrapper = await mountMonitor()

    // Tabs — Monitor.vue has 6 tab panes (metrics, alerts, thresholds, logs, audit, realtime)
    const tabs = wrapper.findAll('.el-tabs__item')
    expect(tabs).toHaveLength(6)
    expect(tabs[0].text()).toBe('指标监控')
    expect(tabs[1].text()).toBe('告警监控')
    expect(tabs[2].text()).toBe('告警阈值')
    expect(tabs[3].text()).toBe('系统日志')
    expect(tabs[4].text()).toBe('审计日志')
    expect(tabs[5].text()).toBe('实时行情')

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
  // 2. Alert acknowledgment
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
  // 3. Tab switching
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
  // 4. Disposes ECharts on unmount
  // -----------------------------------------------------------------------
  it('disposes ECharts on unmount', async () => {
    const wrapper = await mountMonitor()

    wrapper.unmount()

    expect(getChartMock().dispose).toHaveBeenCalled()
  })
})
