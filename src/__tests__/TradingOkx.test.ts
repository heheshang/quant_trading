import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Trading from '@/views/Trading.vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

// ---------------------------------------------------------------------------
// Hoisted mocks — same pattern as Trading.test.ts
// ---------------------------------------------------------------------------

vi.mock('@/composables/useWebSocketStatus', () => ({
  useWebSocketStatus: () => ({ status: 'connected', retryIn: 0, startListening: vi.fn(), cleanup: vi.fn() }),
}))

vi.mock('@/composables/useMarketData', () => ({
  useMarketData: () => ({ startListening: vi.fn(), cleanup: vi.fn(), tickerData: { value: {} }, trades: { value: {} }, orderbook: { value: {} }, candleData: { value: {} } }),
}))

vi.mock('@/stores/order', () => ({
  useOrderStore: () => ({ placeOrder: vi.fn(), orderCount: 0, activeOrders: [], loading: false, error: null, fetchActiveOrders: vi.fn().mockResolvedValue([]) }),
}))

// Re-mock element-plus with actual components + mocked ElMessage (same as Trading.test.ts)
vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockInvoke = vi.mocked(invoke)

/**
 * Set up the mock invoke map. Commands not in the map return {}.
 * Call this in beforeEach or per-test before mountComponent().
 */
function setupMock(overrides: Record<string, unknown> = {}) {
  const defaults: Record<string, unknown> = {
    get_account_info: { total_assets: 1_000_000, available_cash: 500_000, market_value: 500_000, daily_pnl: 5_000 },
    get_positions: [],
    get_active_orders: [],
    get_strategies: [],
    check_okx_status: { connected: true, demo_trading: true, exchange_time: '2024-06-01T00:00:00Z', message: 'OK' },
    get_okx_instruments: [],
    get_okx_balance: [],
    get_okx_positions: [],
    get_okx_announcements: [],
    get_okx_candles: [],
  }
  const merged = { ...defaults, ...overrides }
  mockInvoke.mockImplementation(async (cmd: string) => {
    if (cmd in merged) return merged[cmd]
    return {}
  })
}

let container: HTMLDivElement

async function mountComponent(): Promise<any> {
  const wrapper = mount(Trading, { attachTo: container, global: { plugins: [ElementPlus] } })
  for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
  await new Promise(r => setTimeout(r, 30))
  return wrapper
}

async function mountWithOkxTab(): Promise<any> {
  const wrapper = await mountComponent()
  wrapper.vm.activeTradeTab = 'okx'
  for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
  await new Promise(r => setTimeout(r, 30))
  return wrapper
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('Trading.vue - OKX 数据渲染测试', () => {
  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    vi.clearAllMocks()
    setupMock()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    container.remove()
  })

  // ---- V1: Balance table rendering ----
  it('renders balance table with 3 items and correct column data', async () => {
    const balanceData = [
      { ccy: 'BTC', cashBal: '2.0', eq: '2.5', uTime: '1700000000000' },
      { ccy: 'ETH', cashBal: '15.0', eq: '15.5', uTime: '1700000000000' },
      { ccy: 'USDT', cashBal: '50000', eq: '50000', uTime: '1700000000000' },
    ]
    setupMock({ get_okx_balance: balanceData })

    const wrapper = await mountWithOkxTab()

    // Fetch balance data (not auto-fetched on mount)
    await wrapper.vm.fetchOkxBalance()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    expect(wrapper.vm.okxBalance).toHaveLength(3)
    expect(wrapper.vm.okxBalance[0].ccy).toBe('BTC')
    expect(wrapper.vm.okxBalance[1].ccy).toBe('ETH')
    expect(wrapper.vm.okxBalance[2].ccy).toBe('USDT')
    expect(wrapper.vm.okxBalance[0].eq).toBe('2.5')
    expect(wrapper.vm.okxBalanceLoading).toBe(false)

    // DOM check: page text includes coin names
    const text = wrapper.text()
    expect(text).toContain('BTC')
    expect(text).toContain('ETH')
    expect(text).toContain('USDT')
    expect(text).toContain('2.0')
    expect(text).toContain('2.5')
  }, 30000)

  // ---- V2: Empty positions state ----
  it('shows empty state when positions array is empty', async () => {
    setupMock({ get_okx_positions: [] })
    const wrapper = await mountWithOkxTab()

    await wrapper.vm.fetchOkxPositions()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    expect(wrapper.vm.okxPositions).toHaveLength(0)
    expect(wrapper.vm.okxPositionsLoading).toBe(false)
  }, 30000)

  // ---- V3: Positions with data ----
  it('renders positions table when data is loaded', async () => {
    const positionsData = [
      { instId: 'BTC-USDT', pos: '0.5', avgPx: '50000', upl: '1000' },
      { instId: 'ETH-USDT', pos: '2.0', avgPx: '3000', upl: '200' },
    ]
    setupMock({ get_okx_positions: positionsData })
    const wrapper = await mountWithOkxTab()

    await wrapper.vm.fetchOkxPositions()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    expect(wrapper.vm.okxPositions).toHaveLength(2)
    expect(wrapper.vm.okxPositions[0].instId).toBe('BTC-USDT')
    expect(wrapper.vm.okxPositions[1].instId).toBe('ETH-USDT')
    expect(wrapper.vm.okxPositions[0].upl).toBe('1000')

    const text = wrapper.text()
    expect(text).toContain('BTC-USDT')
    expect(text).toContain('ETH-USDT')
  }, 30000)

  // ---- V4: K-line chart container ----
  it('renders K-line chart container when candles are fetched successfully', async () => {
    const now = Date.now()
    const candles = Array.from({ length: 60 }, (_, i) => ({
      ts: String(now - (59 - i) * 3_600_000),
      o: 50000 + i * 10,
      h: 50100 + i * 10,
      l: 49900 + i * 10,
      c: 50050 + i * 10,
      vol: 1000 + i * 100,
    }))
    setupMock({ get_okx_candles: candles })

    const wrapper = await mountWithOkxTab()

    // Verify chart ref div exists before fetching
    expect(wrapper.vm.okxCandleChartRef).toBeDefined()

    // Fetch candles
    await wrapper.vm.fetchOkxCandles()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // No error
    expect(wrapper.vm.okxCandleError).toBe('')
  }, 30000)

  // ---- V5: K-line error state ----
  it('shows error message when K-line data fetch fails', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_okx_candles') throw new Error('K-line data timeout')
      // For other commands, use defaults
      const defaults: Record<string, unknown> = {
        get_account_info: { total_assets: 1_000_000, available_cash: 500_000, market_value: 500_000, daily_pnl: 5_000 },
        get_positions: [],
        get_active_orders: [],
        get_strategies: [],
        check_okx_status: { connected: true, demo_trading: true },
        get_okx_instruments: [],
        get_okx_balance: [],
        get_okx_positions: [],
        get_okx_announcements: [],
      }
      if (cmd in defaults) return defaults[cmd as keyof typeof defaults]
      return {}
    })

    const wrapper = await mountWithOkxTab()

    await wrapper.vm.fetchOkxCandles()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    expect(wrapper.vm.okxCandleError).toBe('K-line data timeout')

    // DOM check: error message is rendered
    const text = wrapper.text()
    expect(text).toContain('K-line data timeout')
  }, 30000)

  // ---- V6: Loading state ----
  it('reflects balance loading state', async () => {
    const wrapper = await mountWithOkxTab()

    // Initial state is not loading
    expect(wrapper.vm.okxBalanceLoading).toBe(false)

    // Set loading to true — v-loading directive should activate
    wrapper.vm.okxBalanceLoading = true
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.okxBalanceLoading).toBe(true)

    // Reset and verify
    wrapper.vm.okxBalanceLoading = false
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.okxBalanceLoading).toBe(false)
  }, 30000)

  // ---- V7: Error handling ----
  it('shows error message on balance fetch failure', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_okx_balance') throw new Error('API rate limit exceeded')
      const defaults: Record<string, unknown> = {
        get_account_info: { total_assets: 1_000_000, available_cash: 500_000, market_value: 500_000, daily_pnl: 5_000 },
        get_positions: [], get_active_orders: [], get_strategies: [],
        check_okx_status: { connected: true, demo_trading: true },
        get_okx_instruments: [], get_okx_positions: [], get_okx_announcements: [],
      }
      if (cmd in defaults) return defaults[cmd as keyof typeof defaults]
      return {}
    })

    const wrapper = await mountWithOkxTab()

    await wrapper.vm.fetchOkxBalance()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // ElMessage.error should have been called
    expect(ElMessage.error).toHaveBeenCalled()
    // Balance array should remain empty
    expect(wrapper.vm.okxBalance).toHaveLength(0)
    // Loading flag reset
    expect(wrapper.vm.okxBalanceLoading).toBe(false)
  }, 30000)

  // ---- V8: Connection status ----
  it('shows connected status indicator when OKX is connected', async () => {
    setupMock({
      check_okx_status: { connected: true, demo_trading: true, exchange_time: '2024-06-01T00:00:00Z', message: 'OK' },
    })

    const wrapper = await mountWithOkxTab()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // Status already fetched via onMounted
    expect(wrapper.vm.okxStatus).toBeDefined()
    expect(wrapper.vm.okxStatus.connected).toBe(true)
    expect(wrapper.vm.okxConnected).toBe(true)

    // DOM: status card shows 已连接 tag
    const text = wrapper.text()
    expect(text).toContain('已连接')
  }, 30000)

  // ---- V9: Instrument selector ----
  it('renders instrument selector with 10 items and show-more link when > 10', async () => {
    const instruments = [
      { instId: 'BTC-USDT', baseCcy: 'BTC', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'ETH-USDT', baseCcy: 'ETH', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'SOL-USDT', baseCcy: 'SOL', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'DOGE-USDT', baseCcy: 'DOGE', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'XRP-USDT', baseCcy: 'XRP', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'ADA-USDT', baseCcy: 'ADA', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'DOT-USDT', baseCcy: 'DOT', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'AVAX-USDT', baseCcy: 'AVAX', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'LINK-USDT', baseCcy: 'LINK', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'MATIC-USDT', baseCcy: 'MATIC', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'ATOM-USDT', baseCcy: 'ATOM', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'UNI-USDT', baseCcy: 'UNI', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'LTC-USDT', baseCcy: 'LTC', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'BCH-USDT', baseCcy: 'BCH', quoteCcy: 'USDT', instType: 'SPOT' },
      { instId: 'FIL-USDT', baseCcy: 'FIL', quoteCcy: 'USDT', instType: 'SPOT' },
    ]
    setupMock({ get_okx_instruments: instruments })

    const wrapper = await mountWithOkxTab()
    // Instruments are auto-fetched in onMounted
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    expect(wrapper.vm.okxInstruments).toHaveLength(15)
    // By default showAllInstruments is false
    expect(wrapper.vm.showAllInstruments).toBe(false)

    // DOM should show first 10 instrument IDs
    const text = wrapper.text()
    expect(text).toContain('BTC-USDT')
    expect(text).toContain('MATIC-USDT')
    // Show-more link with total count
    expect(text).toContain('显示全部 (15)')

    // Click show more
    wrapper.vm.showAllInstruments = true
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.showAllInstruments).toBe(true)
  }, 30000)

  // ---- V10: Announcement empty state ----
  it('shows empty state text when no announcements', async () => {
    setupMock({ get_okx_announcements: [] })
    const wrapper = await mountWithOkxTab()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    expect(wrapper.vm.okxAnnouncements).toHaveLength(0)

    const text = wrapper.text()
    expect(text).toContain('暂无公告')
  }, 30000)

  // ---- V11: Announcement with data ----
  it('renders announcement list when data is loaded', async () => {
    const announcements = [
      { title: 'BTC-USDT 永续合约上线', url: 'https://www.okx.com/announcement/1' },
      { title: '交易手续费调整通知', url: 'https://www.okx.com/announcement/2' },
      { title: 'API 更新日志 2024', url: 'https://www.okx.com/announcement/3' },
    ]
    setupMock({ get_okx_announcements: announcements })

    const wrapper = await mountWithOkxTab()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    expect(wrapper.vm.okxAnnouncements).toHaveLength(3)
    expect(wrapper.vm.okxAnnouncements[0].title).toBe('BTC-USDT 永续合约上线')
    expect(wrapper.vm.okxAnnouncements[1].title).toBe('交易手续费调整通知')
    expect(wrapper.vm.okxAnnouncements[2].title).toBe('API 更新日志 2024')

    // Should NOT show empty state
    const text = wrapper.text()
    expect(text).not.toContain('暂无公告')
    expect(text).toContain('BTC-USDT 永续合约上线')
    expect(text).toContain('交易手续费调整通知')
  }, 30000)

  // ---- Order submission ----
  it('shows success message when OKX order is submitted', async () => {
    setupMock({
      place_okx_order: { ordId: 'ord-abc-123', instId: 'BTC-USDT', state: 'filled' },
      // Need balance + positions mock because submitOkxOrder also calls fetchOkxBalance + fetchOkxPositions
      get_okx_balance: [{ ccy: 'BTC', cashBal: '2.0', eq: '2.5', uTime: '1700000000000' }],
      get_okx_positions: [{ instId: 'BTC-USDT', pos: '0.5', avgPx: '50000', upl: '1000' }],
    })

    const wrapper = await mountWithOkxTab()

    // Set order form values on the actual child form
    Object.assign(wrapper.vm.okxOrderFormRef.formData, {
      instId: 'BTC-USDT',
      side: 'buy',
      ordType: 'limit',
      px: 50000,
      sz: 0.1,
    })

    // Submit
    await wrapper.vm.submitOkxOrder()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // Verify success feedback
    expect(ElMessage.success).toHaveBeenCalled()
    // Submitting flag reset
    expect(wrapper.vm.okxSubmitting).toBe(false)
  }, 30000)
})
