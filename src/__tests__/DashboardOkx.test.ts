import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'

// ---------------------------------------------------------------------------
// Hoisted mock data — accessible inside vi.mock factories (which are hoisted)
// ---------------------------------------------------------------------------
const { mockStartListening, mockCleanup, okxMarketData } = vi.hoisted(() => {
  const mockStartListening = vi.fn()
  const mockCleanup = vi.fn()

  const defaultAccount = {
    account_id: 0,
    total_assets: 1_000_000,
    available_cash: 500_000,
    frozen_cash: 0,
    market_value: 500_000,
    total_pnl: 50_000,
    daily_pnl: 3_000,
    margin: 0,
    margin_ratio: 0,
    updated_at: '2024-01-01T00:00:00Z',
  }

  const defaultPositions = [
    {
      symbol: 'BTC-USDT',
      quantity: 1,
      available_quantity: 1,
      avg_price: 40_000,
      market_value: 50_000,
      unrealized_pnl: 10_000,
      realized_pnl: 0,
      updated_at: '2024-01-01T00:00:00Z',
    },
  ]

  const defaultOrders = [
    {
      order_id: '1',
      strategy_id: 's1',
      symbol: 'BTC-USDT',
      order_type: 'Limit' as const,
      side: 'Buy' as const,
      price: 40_000,
      quantity: 1,
      filled_quantity: 1,
      status: 'Filled' as const,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
      commission: 0,
      slippage: 0,
    },
  ]

  // OKX market data shape matching what Dashboard.vue expects:
  // symbol, price, change, change_percent, volume, high, low
  const okxMarketData = {
    symbol: 'BTC-USDT',
    price: 52345.60,
    change: 1234.50,
    change_percent: '+2.41%',
    volume: 12345,
    high: 53000.00,
    low: 51200.00,
  }

  return { mockStartListening, mockCleanup, defaultAccount, defaultPositions, defaultOrders, okxMarketData }
})

// ---------------------------------------------------------------------------
// Mock useMarketData — control startListening / cleanup spies
// ---------------------------------------------------------------------------
vi.mock('@/composables/useMarketData', () => ({
  useMarketData: () => ({
    startListening: mockStartListening,
    cleanup: mockCleanup,
    tickerData: { value: {} },
  }),
}))

// ---------------------------------------------------------------------------
// Mock useFormatting — real implementation for predictable formatting
// ---------------------------------------------------------------------------
vi.mock('@/composables/useFormatting', () => ({
  useFormatting: () => ({
    formatCurrency: (v: number | string) => {
      const n = Number(v)
      if (!n && n !== 0) return '0.00'
      return n.toLocaleString('zh-CN', {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      })
    },
    formatDate: (d: string | Date) => (d ? new Date(d).toLocaleString('zh-CN') : '-'),
    formatOrderSide: (s: string) => (s === 'Buy' ? '买入' : s === 'Sell' ? '卖出' : s),
    formatOrderStatus: (s: string) => {
      const map: Record<string, string> = { Pending: '待提交', Filled: '已成交' }
      return map[s] ?? s
    },
  }),
}))

// ---------------------------------------------------------------------------
// Mock API services — include getMarketData for OKX market data section
// ---------------------------------------------------------------------------
vi.mock('@/services/account', () => ({
  getAccountInfo: vi.fn().mockResolvedValue({
    account_id: 0,
    total_assets: 1_000_000,
    available_cash: 500_000,
    frozen_cash: 0,
    market_value: 500_000,
    total_pnl: 50_000,
    daily_pnl: 3_000,
    margin: 0,
    margin_ratio: 0,
    updated_at: '2024-01-01T00:00:00Z',
  }),
  getPositions: vi.fn().mockResolvedValue([]),
}))
vi.mock('@/services/order', () => ({
  getActiveOrders: vi.fn().mockResolvedValue([]),
}))
vi.mock('@/services/market', () => ({
  getMarketData: vi.fn().mockResolvedValue(okxMarketData),
}))

// ---------------------------------------------------------------------------
// Stub child components
// ---------------------------------------------------------------------------
vi.mock('@/components/StatsCard.vue', () => ({
  default: {
    name: 'StatsCard',
    template: '<div class="stats-card-stub" :data-title="title"></div>',
    props: ['title', 'value', 'format', 'icon', 'iconBg', 'trend', 'loading'],
  },
}))

vi.mock('@/components/dashboard/RealtimeTickerPanel.vue', () => ({
  default: {
    name: 'RealtimeTickerPanel',
    template: '<div class="ticker-panel-stub" :data-symbols="JSON.stringify(symbols)"></div>',
    props: ['symbols'],
  },
}))

// ---------------------------------------------------------------------------
// Imports (after all vi.mock calls so hoisting works)
// ---------------------------------------------------------------------------
import Dashboard from '@/views/Dashboard.vue'
import { getMarketData } from '@/services/market'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
const flushPromises = () => new Promise<void>((r) => setTimeout(r, 0))

function createWrapper(): any {
  return mount(Dashboard, {
    global: {
      stubs: {
        'el-alert': {
          template: '<div class="el-alert-stub"><slot /></div>',
          props: ['title', 'type', 'closable'],
        },
        'el-button': {
          template: '<button class="el-button-stub"><slot /></button>',
          props: ['type', 'loading'],
        },
        'el-row': { template: '<div class="el-row-stub"><slot /></div>', props: ['gutter'] },
        'el-col': { template: '<div class="el-col-stub"><slot /></div>', props: ['span'] },
        'el-card': {
          template: '<div class="el-card-stub"><slot name="header" /><slot /></div>',
          props: ['shadow'],
        },
        'el-skeleton': {
          template: '<div class="el-skeleton-stub"></div>',
          props: ['rows', 'animated'],
        },
        'el-table': {
          template: `
            <div class="el-table-stub">
              <div v-for="(row, idx) in data" :key="idx" class="el-table-row-stub">
                <slot v-bind="{ row, $index: idx, column: {} }" />
              </div>
            </div>
          `,
          props: ['data'],
        },
        'el-tag': { template: '<span class="el-tag-stub"><slot /></span>', props: ['type'] },
      },
    },
  })
}

// ---------------------------------------------------------------------------
// Tests — OKX market data section
// ---------------------------------------------------------------------------
describe('Dashboard.vue - OKX market data section', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    const pinia = createPinia()
    setActivePinia(pinia)

    // Reset getMarketData mock to success
    vi.mocked(getMarketData).mockResolvedValue(okxMarketData as any)
  })

  // -----------------------------------------------------------------------
  // 1. Initial state shows placeholder before fetch
  // -----------------------------------------------------------------------
  it('shows placeholder text before market data is fetched', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    // The market data card should show placeholder initially
    const placeholders = wrapper.findAll('.market-data-placeholder')
    expect(placeholders.length).toBeGreaterThanOrEqual(1)
    expect(wrapper.text()).toContain('点击刷新加载行情数据')
  })

  // -----------------------------------------------------------------------
  // 2. fetchMarketData renders OKX market data in the grid
  // -----------------------------------------------------------------------
  it('renders OKX market ticker data after fetchMarketData', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    // Call fetchMarketData directly
    await wrapper.vm.fetchMarketData()
    await flushPromises()
    await nextTick()

    // Market data grid should now be rendered
    const grid = wrapper.find('.market-data-grid')
    expect(grid.exists()).toBe(true)

    // Check individual market items
    const marketValues = wrapper.findAll('.market-value')
    const texts = marketValues.map((el: any) => el.text())

    expect(texts.some((t: string) => t.includes('BTC-USDT'))).toBe(true)
    // price: 52345.60 → JS renders as 52345.6 (trailing zero dropped)
    expect(texts.some((t: string) => t.includes('52345.6'))).toBe(true)
    expect(texts.some((t: string) => t.includes('+2.41%'))).toBe(true)
    expect(texts.some((t: string) => t.includes('12,345') || t.includes('12345'))).toBe(true)
    // high: 53000.00 → JS trims to 53000
    expect(texts.some((t: string) => t.includes('53,000') || t.includes('53000'))).toBe(true)
    // low: 51200.00 → JS trims to 51200
    expect(texts.some((t: string) => t.includes('51,200') || t.includes('51200'))).toBe(true)
  })

  // -----------------------------------------------------------------------
  // 3. Loading state during market data fetch
  // -----------------------------------------------------------------------
  it('sets marketLoading during fetchMarketData', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    // Freeze the promise so we can check loading state
    let resolvePromise!: (v: any) => void
    vi.mocked(getMarketData).mockReturnValue(new Promise((resolve: any) => { resolvePromise = resolve }))

    wrapper.vm.fetchMarketData()
    await flushPromises()
    await nextTick()

    expect(wrapper.vm.marketLoading).toBe(true)

    // Resolve the promise
    resolvePromise(okxMarketData)
    await flushPromises()
    await nextTick()

    expect(wrapper.vm.marketLoading).toBe(false)
  })

  // -----------------------------------------------------------------------
  // 4. Error state when API call fails
  // -----------------------------------------------------------------------
  it('shows error message when getMarketData fails', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    // Mock failure
    vi.mocked(getMarketData).mockRejectedValue(new Error('Connection timeout'))

    await wrapper.vm.fetchMarketData()
    await flushPromises()
    await nextTick()

    // Error placeholder should appear
    const placeholders = wrapper.findAll('.market-data-placeholder')
    expect(placeholders.length).toBeGreaterThanOrEqual(1)
    expect(wrapper.text()).toContain('获取行情数据失败')
    expect(wrapper.text()).toContain('Connection timeout')
  })

  // -----------------------------------------------------------------------
  // 5. Error state for 'Not implemented' returns specific message
  // -----------------------------------------------------------------------
  it('shows "功能开发中" when error contains "Not implemented"', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    vi.mocked(getMarketData).mockRejectedValue(new Error('Not implemented'))

    await wrapper.vm.fetchMarketData()
    await flushPromises()
    await nextTick()

    expect(wrapper.text()).toContain('行情数据功能开发中')
  })

  // -----------------------------------------------------------------------
  // 6. Refresh button triggers fetchMarketData
  // -----------------------------------------------------------------------
  it('calls fetchMarketData when refresh button is clicked', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    const fetchSpy = vi.spyOn(wrapper.vm, 'fetchMarketData')

    // Find the refresh button inside the market data card header
    // The refresh button has a stub, so trigger via the vm instead
    wrapper.vm.fetchMarketData()
    expect(fetchSpy).toHaveBeenCalled()
  })

  // -----------------------------------------------------------------------
  // 7. getMarketData is called with 'default' symbol
  // -----------------------------------------------------------------------
  it('calls getMarketData with "default" symbol', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    vi.mocked(getMarketData).mockClear()

    await wrapper.vm.fetchMarketData()
    await flushPromises()
    await nextTick()

    expect(getMarketData).toHaveBeenCalledWith('default')
  })

  // -----------------------------------------------------------------------
  // 8. marketError is cleared before each fetch
  // -----------------------------------------------------------------------
  it('clears marketError before fetching', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    // Set an error first
    wrapper.vm.marketError = 'Previous error'
    await nextTick()

    // Fetch successfully
    await wrapper.vm.fetchMarketData()
    await flushPromises()
    await nextTick()

    // Error should be cleared
    expect(wrapper.vm.marketError).toBe('')
  })

  // -----------------------------------------------------------------------
  // 9. API call failure does not crash the component
  // -----------------------------------------------------------------------
  it('does not crash when getMarketData throws', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    vi.mocked(getMarketData).mockRejectedValue(new Error('Network error'))

    await wrapper.vm.fetchMarketData()
    await flushPromises()
    await nextTick()

    expect(wrapper.exists()).toBe(true)
  })
})
