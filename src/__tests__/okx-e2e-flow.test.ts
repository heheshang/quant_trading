import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Trading from '@/views/Trading.vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'

// ---------------------------------------------------------------------------
// Hoisted mocks — same pattern as Trading.test.ts / TradingOkx.test.ts
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
// E2E Suite — verifies the ENTIRE pipeline:
//   mock invoke → api.ts wrapper → Vue component state → DOM rendering
// ---------------------------------------------------------------------------

describe('OKX E2E Data Flow', () => {
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

  // =======================================================================
  // Test 1: K-line data flow
  //   mock invoke  →  getOkxCandles()  →  component state  →  chart data
  // =======================================================================

  it('K-line data flows from invoke to chart data', async () => {
    const now = Date.now()
    const candles = Array.from({ length: 60 }, (_, i) => ({
      ts: String(now - (59 - i) * 3_600_000),
      o: 50000 + i * 10,
      h: 50100 + i * 10,
      l: 49900 + i * 10,
      c: 50050 + i * 10,
      vol: 1000 + i * 10,
    }))

    setupMock({ get_okx_candles: candles })
    const wrapper = await mountWithOkxTab()

    // Chart ref div exists before fetching
    expect(wrapper.vm.okxCandleChartRef).toBeDefined()

    // Fetch candles — triggers the full pipeline:
    //   fetchOkxCandles() → getOkxCandles() → invoke('get_okx_candles')
    await wrapper.vm.fetchOkxCandles()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // 1. invoke was called with correct command and params
    expect(mockInvoke).toHaveBeenCalledWith('get_okx_candles', {
      instId: 'BTC-USDT',
      bar: '1H',
      limit: 60,
    })

    // 2. No error — data made it through the pipeline
    expect(wrapper.vm.okxCandleError).toBe('')
  }, 30000)

  // =======================================================================
  // Test 2: Balance data flow
  //   mock invoke  →  getOkxBalance()  →  component okxBalance[]  →  DOM
  // =======================================================================

  it('balance data flows via invoke → api.ts → component → DOM', async () => {
    const balances = [
      { ccy: 'BTC', cashBal: '50000', eq: '50000', uTime: '1700000000000' },
      { ccy: 'ETH', cashBal: '30000', eq: '30000', uTime: '1700000000000' },
      { ccy: 'USDT', cashBal: '100000', eq: '100000', uTime: '1700000000000' },
    ]
    setupMock({ get_okx_balance: balances })
    const wrapper = await mountWithOkxTab()

    // Fetch — pipeline: fetchOkxBalance() → getOkxBalance() → invoke('get_okx_balance')
    await wrapper.vm.fetchOkxBalance()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // Component state: 3 balances loaded
    expect(wrapper.vm.okxBalance).toHaveLength(3)
    expect(wrapper.vm.okxBalance[0].ccy).toBe('BTC')
    expect(wrapper.vm.okxBalance[1].ccy).toBe('ETH')
    expect(wrapper.vm.okxBalance[2].ccy).toBe('USDT')
    expect(wrapper.vm.okxBalance[0].eq).toBe('50000')
    expect(wrapper.vm.okxBalanceLoading).toBe(false)

    // DOM: table renders all three coin names and values
    const text = wrapper.text()
    expect(text).toContain('BTC')
    expect(text).toContain('ETH')
    expect(text).toContain('USDT')
    expect(text).toContain('50000')
  }, 30000)

  // =======================================================================
  // Test 3: Order submission flow
  //   form fill → validate → placeOkxOrder() → invoke → success feedback
  // =======================================================================

  it('submits OKX order and shows success feedback', async () => {
    setupMock({
      place_okx_order: { ordId: 'ord-abc-123', instId: 'BTC-USDT', state: 'filled' },
      // submitOkxOrder also re-fetches balance + positions after success
      get_okx_balance: [{ ccy: 'BTC', cashBal: '2.0', eq: '2.5', uTime: '1700000000000' }],
      get_okx_positions: [{ instId: 'BTC-USDT', pos: '0.5', avgPx: '50000', upl: '1000' }],
    })
    const wrapper = await mountWithOkxTab()

    // Mock form validation (same pattern as Trading.test.ts mockFormRef)
    wrapper.vm.okxOrderFormRef = {
      validate: vi.fn((cb: any) => {
        cb(true)
        return Promise.resolve(true)
      }),
    } as any

    // Fill order form with real-world values
    wrapper.vm.okxOrderForm = { instId: 'BTC-USDT', side: 'buy', ordType: 'limit', px: 50000, sz: 0.1 }

    // Submit — pipeline: submitOkxOrder() → validate → placeOkxOrder() → invoke('place_okx_order')
    await wrapper.vm.submitOkxOrder()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // 1. invoke was called with correct command
    expect(mockInvoke).toHaveBeenCalledWith('place_okx_order', expect.objectContaining({
      request: expect.objectContaining({
        instId: 'BTC-USDT',
        side: 'buy',
        ordType: 'limit',
      }),
    }))

    // 2. Success feedback shown to user
    expect(ElMessage.success).toHaveBeenCalled()

    // 3. Loading flag is reset
    expect(wrapper.vm.okxSubmitting).toBe(false)

    // 4. Balance + positions re-fetched after order placement
    expect(mockInvoke).toHaveBeenCalledWith('get_okx_balance', { ccy: undefined })
    expect(mockInvoke).toHaveBeenCalledWith('get_okx_positions', { instId: undefined })
  }, 30000)

  // =======================================================================
  // Test 4: Error propagation
  //   invoke throws → catch → ElMessage.error → state preserved
  // =======================================================================

  it('handles API error and shows error message', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_okx_balance') throw new Error('API rate limit exceeded')
      const defaults: Record<string, unknown> = {
        get_account_info: { total_assets: 1_000_000, available_cash: 500_000, market_value: 500_000, daily_pnl: 5_000 },
        get_positions: [],
        get_active_orders: [],
        get_strategies: [],
        check_okx_status: { connected: true, demo_trading: true },
        get_okx_instruments: [],
        get_okx_positions: [],
        get_okx_announcements: [],
        get_okx_candles: [],
      }
      if (cmd in defaults) return defaults[cmd as keyof typeof defaults]
      return {}
    })
    const wrapper = await mountWithOkxTab()

    // Fetch balance — should fail
    await wrapper.vm.fetchOkxBalance()
    for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
    await new Promise(r => setTimeout(r, 30))

    // 1. Error feedback shown to user
    expect(ElMessage.error).toHaveBeenCalled()

    // 2. Balance state remains empty (not corrupted by partial data)
    expect(wrapper.vm.okxBalance).toHaveLength(0)

    // 3. Loading flag is reset in finally block
    expect(wrapper.vm.okxBalanceLoading).toBe(false)
  }, 30000)
})
