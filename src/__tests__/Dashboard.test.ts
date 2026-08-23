import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'

// ---------------------------------------------------------------------------
// Hoisted mock data — accessible inside vi.mock factories (which are hoisted)
// ---------------------------------------------------------------------------
const {
  defaultAccount,
  defaultPositions,
  defaultOrders,
} = vi.hoisted(() => {
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
    {
      symbol: 'ETH-USDT',
      quantity: 10,
      available_quantity: 10,
      avg_price: 2_000,
      market_value: 25_000,
      unrealized_pnl: 5_000,
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

  return { defaultAccount, defaultPositions, defaultOrders }
})

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
// Mock API services — called by store fetch methods in onMounted
// ---------------------------------------------------------------------------
vi.mock('@/services/account', () => ({
  getAccountInfo: vi.fn().mockResolvedValue(defaultAccount),
  getPositions: vi.fn().mockResolvedValue(defaultPositions),
}))
vi.mock('@/services/order', () => ({
  getActiveOrders: vi.fn().mockResolvedValue(defaultOrders),
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

// ---------------------------------------------------------------------------
// Imports (after all vi.mock calls so hoisting works)
// ---------------------------------------------------------------------------
import Dashboard from '@/views/Dashboard.vue'
import { useAccountStore } from '@/stores/account'

import * as echarts from 'echarts'
import { getAccountInfo, getPositions } from '@/services/account'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
const flushPromises = () => new Promise<void>((r) => setTimeout(r, 0))

function createWrapper() {
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
        // Use a proper el-table stub that iterates data and provides row scope
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
// Tests
// ---------------------------------------------------------------------------
describe('Dashboard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    const pinia = createPinia()
    setActivePinia(pinia)

    // Reset API mocks to defaults for each test
    vi.mocked(getAccountInfo).mockResolvedValue(defaultAccount)
    vi.mocked(getPositions).mockResolvedValue(defaultPositions)
  })

  // -----------------------------------------------------------------------
  // 1. Component mounts and renders the dashboard layout
  // -----------------------------------------------------------------------
  it('mounts and renders the dashboard layout', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    expect(wrapper.find('.dashboard').exists()).toBe(true)
    expect(wrapper.find('.dashboard-header').exists()).toBe(true)
    expect(wrapper.find('.dashboard-header h2').text()).toBe('仪表盘')
    expect(wrapper.find('.el-button-stub').exists()).toBe(true)
  })

  // -----------------------------------------------------------------------
  // 3. PnL card displays total_pnl and unrealized_pnl from accountStore
  // -----------------------------------------------------------------------
  it('displays total_pnl and unrealized_pnl in the PnL card', async () => {
    const accountStore = useAccountStore()
    accountStore.accountInfo = defaultAccount
    accountStore.positions = defaultPositions

    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    const pnlCard = wrapper.find('.pnl-card')
    expect(pnlCard.exists()).toBe(true)

    // totalPnl = 50000 → "50,000.00", unrealizedPnl = 10000 + 5000 = 15000 → "15,000.00"
    const pnlValues = wrapper.findAll('.pnl-value')
    expect(pnlValues).toHaveLength(2)
    expect(pnlValues[0].text()).toContain('50,000.00')
    expect(pnlValues[1].text()).toContain('15,000.00')
  })

  // -----------------------------------------------------------------------
  // 4. PnL colors: positive green, negative red (Vue renders hex as rgb())
  // -----------------------------------------------------------------------
  it('applies green for positive PnL values', async () => {
    vi.mocked(getAccountInfo).mockResolvedValue({ ...defaultAccount, total_pnl: 50_000 })
    vi.mocked(getPositions).mockResolvedValue([
      { symbol: 'BTC-USDT', quantity: 1, available_quantity: 1, avg_price: 40000, market_value: 50000, unrealized_pnl: 10000, realized_pnl: 0, updated_at: '2024-01-01T00:00:00Z' },
    ])

    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    const pnlValues = wrapper.findAll('.pnl-value')
    expect(pnlValues[0].attributes('style')).toContain('var(--color-success)')
    expect(pnlValues[1].attributes('style')).toContain('var(--color-success)')
  })

  it('applies red for negative PnL values', async () => {
    vi.mocked(getAccountInfo).mockResolvedValue({ ...defaultAccount, total_pnl: -10_000 })
    vi.mocked(getPositions).mockResolvedValue([
      { symbol: 'BTC-USDT', quantity: 1, available_quantity: 1, avg_price: 40000, market_value: 50000, unrealized_pnl: -5000, realized_pnl: 0, updated_at: '2024-01-01T00:00:00Z' },
    ])

    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    const pnlValues = wrapper.findAll('.pnl-value')
    expect(pnlValues[0].attributes('style')).toContain('var(--color-danger)')
    expect(pnlValues[1].attributes('style')).toContain('var(--color-danger)')
  })

  it('applies different colors when total and unrealized PnL have opposite signs', async () => {
    vi.mocked(getAccountInfo).mockResolvedValue({ ...defaultAccount, total_pnl: 50_000 })
    vi.mocked(getPositions).mockResolvedValue([
      { symbol: 'BTC-USDT', quantity: 1, available_quantity: 1, avg_price: 40000, market_value: 50000, unrealized_pnl: -10000, realized_pnl: 0, updated_at: '2024-01-01T00:00:00Z' },
    ])

    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    const pnlValues = wrapper.findAll('.pnl-value')
    expect(pnlValues[0].attributes('style')).toContain('var(--color-success)')
    expect(pnlValues[1].attributes('style')).toContain('var(--color-danger)')
  })

  // -----------------------------------------------------------------------
  // 7. ECharts equity/position charts are initialized
  // -----------------------------------------------------------------------
  it('initializes ECharts equity and position charts on mount', async () => {
    const accountStore = useAccountStore()
    accountStore.accountInfo = defaultAccount
    accountStore.positions = defaultPositions

    createWrapper()
    await flushPromises()
    await nextTick()

    // initCharts() is called in onMounted after data fetch + nextTick
    const initCalls = vi.mocked(echarts.init).mock.calls.length
    expect(initCalls).toBeGreaterThanOrEqual(2)
  })

  // -----------------------------------------------------------------------
  // 8. 30s interval for REST refresh is set/cleared with lifecycle
  // -----------------------------------------------------------------------
  it('sets a 30s refresh interval on mount', async () => {
    const setIntervalSpy = vi.spyOn(globalThis, 'setInterval')

    createWrapper()
    await flushPromises()
    await nextTick()

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 30_000)
    setIntervalSpy.mockRestore()
  })

  it('clears the refresh interval on unmount', async () => {
    const clearIntervalSpy = vi.spyOn(globalThis, 'clearInterval')

    const wrapper = createWrapper()
    await flushPromises()
    await nextTick()

    wrapper.unmount()

    expect(clearIntervalSpy).toHaveBeenCalled()
    clearIntervalSpy.mockRestore()
  })
})
