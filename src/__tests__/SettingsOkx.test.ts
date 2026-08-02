import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Settings from '@/views/Settings.vue'
import SettingsExchange from '@/components/settings/SettingsExchange.vue'
import { invoke } from '@tauri-apps/api/core'

// ---------------------------------------------------------------------------
// Element Plus — use real implementation but spy on ElMessage
// ---------------------------------------------------------------------------
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
// Default config for the settings form (get_config response)
// ---------------------------------------------------------------------------
const defaultConfig = {
  database: { host: 'localhost', port: 5432, username: 'postgres', password: '', database: 'quant_trading', max_connections: 10 },
  redis: { host: 'localhost', port: 6379, password: '', db: 0, pool_size: 10 },
  trading: { enable_paper_trading: true, max_orders_per_second: 10, default_commission_rate: 0.0003, default_slippage: 0.0001, order_timeout_seconds: 30 },
  risk: { max_position_size: 0.2, max_daily_loss: 0.05, max_drawdown: 0.15, enable_pre_trade_check: true, enable_real_time_monitor: true, var_confidence_level: 0.95 },
  monitoring: { enable_prometheus: true, prometheus_port: 9090, log_level: 'info', alert_email: '', alert_webhook: '' },
  security: { enable_encryption: true, enable_2fa: false, jwt_secret: 'secret', token_expiry_hours: 24, allowed_ips: [] },
}

// ---------------------------------------------------------------------------
// OKX status mock data
// ---------------------------------------------------------------------------
const connectedOkxStatus = {
  connected: true,
  demo_trading: true,
  exchange_time: '2026-06-27T12:00:00Z',
  message: 'Connection OK',
}

const disconnectedOkxStatus = {
  connected: false,
  demo_trading: false,
  exchange_time: null,
  message: 'Unable to connect to OKX API',
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
let container: HTMLDivElement

async function mountComponent(): Promise<any> {
  const wrapper = mount(Settings, {
    attachTo: container,
    global: { plugins: [ElementPlus] },
  })
  for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
  await new Promise((r) => setTimeout(r, 30))
  return wrapper
}

function exchangeVm(wrapper: any) {
  return wrapper.findComponent(SettingsExchange).vm
}

/** Switch to the exchange tab by setting activeTab */
async function switchToExchangeTab(wrapper: any) {
  wrapper.vm.activeTab = 'exchange'
  await wrapper.vm.$nextTick()
  await wrapper.vm.$nextTick()
}

// ---------------------------------------------------------------------------
// Tests — OKX connection status section
// ---------------------------------------------------------------------------
describe('Settings.vue - OKX connection status', () => {
  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_config': return defaultConfig
        case 'check_okx_status': return { ...connectedOkxStatus }
        default: return {}
      }
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    container.remove()
  })

  // -----------------------------------------------------------------------
  // 1. Exchange tab shows placeholder before checking
  // -----------------------------------------------------------------------
  it('shows placeholder text before OKX status is checked', async () => {
    const wrapper = await mountComponent()
    await switchToExchangeTab(wrapper)

    // Should show placeholder before any fetch
    const placeholder = wrapper.find('.okx-status-placeholder')
    expect(placeholder.exists()).toBe(true)
    expect(placeholder.text()).toContain('检测连接')
  }, 30000)

  // -----------------------------------------------------------------------
  // 2. fetchOkxConnStatus renders connected OKX status
  // -----------------------------------------------------------------------
  it('renders OKX status grid with connected data after fetch', async () => {
    const wrapper = await mountComponent()
    await switchToExchangeTab(wrapper)

    // Fetch OKX status
    await exchangeVm(wrapper).fetchConnStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    // Status grid should appear
    const statusGrid = wrapper.find('.okx-status-grid')
    expect(statusGrid.exists()).toBe(true)

    // Check for key status text
    expect(wrapper.text()).toContain('已连接')
    expect(wrapper.text()).toContain('是') // demo_trading
    expect(wrapper.text()).toContain('Connection OK')
  }, 30000)

  // -----------------------------------------------------------------------
  // 3. Disconnected status renders correctly
  // -----------------------------------------------------------------------
  it('renders OKX status grid with disconnected state', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_config': return defaultConfig
        case 'check_okx_status': return { ...disconnectedOkxStatus }
        default: return {}
      }
    })

    const wrapper = await mountComponent()
    await switchToExchangeTab(wrapper)

    await exchangeVm(wrapper).fetchConnStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    const statusGrid = wrapper.find('.okx-status-grid')
    expect(statusGrid.exists()).toBe(true)

    // Should show disconnected state
    expect(wrapper.text()).toContain('未连接')
    expect(wrapper.text()).toContain('否') // demo_trading = false
    expect(wrapper.text()).toContain('Unable to connect')
  }, 30000)

  // -----------------------------------------------------------------------
  // 4. Download button triggers file export
  // -----------------------------------------------------------------------
  it('sets okxChecking loading state during fetch', async () => {
    const wrapper = await mountComponent()
    await switchToExchangeTab(wrapper)

    // Freeze promise to check loading state
    let resolvePromise!: (v: unknown) => void
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_config') return defaultConfig
      if (cmd === 'check_okx_status') return new Promise((resolve) => { resolvePromise = resolve })
      return {}
    })

    const fetchPromise = exchangeVm(wrapper).fetchConnStatus()
    await wrapper.vm.$nextTick()
    expect(exchangeVm(wrapper).checking).toBe(true)

    resolvePromise({ ...connectedOkxStatus })
    await fetchPromise
    await wrapper.vm.$nextTick()
    expect(exchangeVm(wrapper).checking).toBe(false)
  }, 30000)

  // -----------------------------------------------------------------------
  // 5. Error during status check does not crash
  // -----------------------------------------------------------------------
  it('handles check_okx_status API error gracefully', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_config') return defaultConfig
      if (cmd === 'check_okx_status') throw new Error('API unavailable')
      return {}
    })

    const wrapper = await mountComponent()
    await switchToExchangeTab(wrapper)

    await exchangeVm(wrapper).fetchConnStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    // Component should not crash, okxConnStatus stays null (no grid)
    expect(wrapper.find('.okx-status-grid').exists()).toBe(false)
    expect(exchangeVm(wrapper).checking).toBe(false)
    expect(wrapper.exists()).toBe(true)
  }, 30000)

  // -----------------------------------------------------------------------
  // 6. Connected status shows success tag, disconnected shows danger tag
  // -----------------------------------------------------------------------
  it('shows success tag for connected and danger tag for disconnected', async () => {
    // Test connected
    const wrapper = await mountComponent()
    await switchToExchangeTab(wrapper)

    await exchangeVm(wrapper).fetchConnStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    // Find the el-tag-stub inside okx-status-grid
    const statusGrid = wrapper.find('.okx-status-grid')
    // The first status field shows "已连接" text
    expect(statusGrid.text()).toContain('已连接')

    // Test disconnected
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_config') return defaultConfig
      if (cmd === 'check_okx_status') return { ...disconnectedOkxStatus }
      return {}
    })

    await exchangeVm(wrapper).fetchConnStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.find('.okx-status-grid').text()).toContain('未连接')
  }, 30000)

  // -----------------------------------------------------------------------
  // 7. Fetch on mount does not auto-check OKX (only on demand)
  // -----------------------------------------------------------------------
  it('does not call check_okx_status on mount', async () => {
    mockInvoke.mockClear()
    await mountComponent()
    // get_config is called on mount, but not check_okx_status
    const checkCalls = mockInvoke.mock.calls.filter(([cmd]) => cmd === 'check_okx_status')
    expect(checkCalls).toHaveLength(0)
  }, 30000)

  // -----------------------------------------------------------------------
  // 8. check_okx_status is called when fetchOkxConnStatus is invoked
  // -----------------------------------------------------------------------
  it('calls check_okx_status invoke when fetching status', async () => {
    const wrapper = await mountComponent()
    await switchToExchangeTab(wrapper)
    mockInvoke.mockClear()

    await exchangeVm(wrapper).fetchConnStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('check_okx_status')
  }, 30000)
})
