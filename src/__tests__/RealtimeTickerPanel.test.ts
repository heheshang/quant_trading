import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { shallowRef, computed } from 'vue'
import type { WsTicker } from '@/services/types'

// ---------------------------------------------------------------------------
// Mock useMarketData — we control tickerData via a module-level shallowRef
// ---------------------------------------------------------------------------
const mockTickerData = shallowRef<Record<string, WsTicker>>({})

vi.mock('@/composables/useMarketData', () => ({
  useMarketData: () => ({
    tickerData: computed(() => mockTickerData.value),
  }),
}))

import RealtimeTickerPanel from '@/components/dashboard/RealtimeTickerPanel.vue'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function makeTicker(overrides: Partial<WsTicker> = {}): WsTicker {
  return {
    inst_id: 'BTC-USDT',
    last: '50000.123456',
    last_sz: '0.001',
    ask_px: '50001.00',
    ask_sz: '1.5',
    bid_px: '49999.00',
    bid_sz: '2.0',
    open24h: '48000.00',
    high24h: '51000.00',
    low24h: '47000.00',
    vol24h: '1000.5',
    ts: '1700000000000',
    ...overrides,
  }
}

function setTickerData(data: Record<string, WsTicker>): void {
  mockTickerData.value = { ...data }
}

function mountPanel(props: { symbols: string[] }) {
  return mount(RealtimeTickerPanel, {
    props,
    global: {
      stubs: {
        'el-card': {
          template: '<div class="el-card"><slot name="header"/><slot/></div>',
        },
        'el-empty': {
          template: '<div class="el-empty"><div class="el-empty__description">{{ $attrs.description }}</div></div>',
        },
      },
    },
  })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
describe('RealtimeTickerPanel', () => {
  beforeEach(() => {
    setTickerData({})
  })

  // -- Empty state ----------------------------------------------------------

  it('renders "无持仓交易对" when symbols prop is empty array', () => {
    const wrapper = mountPanel({ symbols: [] })

    const emptyEl = wrapper.find('.el-empty')
    expect(emptyEl.exists()).toBe(true)
    expect(emptyEl.find('.el-empty__description').text()).toBe('无持仓交易对')
  })

  // -- Row count ------------------------------------------------------------

  it('renders the correct number of rows when given symbols', () => {
    const symbols = ['BTC-USDT', 'ETH-USDT', 'SOL-USDT']
    const wrapper = mountPanel({ symbols })

    const rows = wrapper.findAll('.ticker-row')
    expect(rows).toHaveLength(3)
  })

  // -- No ticker data → fallback --------------------------------------------

  it('displays "-" for price and high/low when no ticker data is available', () => {
    const wrapper = mountPanel({ symbols: ['BTC-USDT'] })

    const row = wrapper.find('.ticker-row')

    // Price column shows "-"
    expect(row.find('.col-price').text()).toBe('-')

    // High / Low columns show "-"
    const highLowValues = row.findAll('.high-low-row .value')
    expect(highLowValues[0].text()).toBe('-')
    expect(highLowValues[1].text()).toBe('-')
  })

  it('shows "0.00%" in change badge when no ticker data is available', () => {
    const wrapper = mountPanel({ symbols: ['BTC-USDT'] })

    // computeChangePct returns 0 → isPositive=true → "+0.00%"
    expect(wrapper.find('.change-badge').text()).toBe('+0.00%')
  })

  // -- Up / down CSS classes ------------------------------------------------

  it('applies "up" CSS class to price and change badge when changePct is positive', () => {
    setTickerData({
      'BTC-USDT': makeTicker({ last: '51000.00', open24h: '48000.00' }),
    })

    const wrapper = mountPanel({ symbols: ['BTC-USDT'] })

    const priceCell = wrapper.find('.ticker-row .col-price')
    expect(priceCell.classes()).toContain('up')
    expect(priceCell.classes()).not.toContain('down')

    const changeBadge = wrapper.find('.change-badge')
    expect(changeBadge.classes()).toContain('up')
    expect(changeBadge.classes()).not.toContain('down')
  })

  it('applies "down" CSS class to price and change badge when changePct is negative', () => {
    setTickerData({
      'BTC-USDT': makeTicker({ last: '46000.00', open24h: '48000.00' }),
    })

    const wrapper = mountPanel({ symbols: ['BTC-USDT'] })

    const priceCell = wrapper.find('.ticker-row .col-price')
    expect(priceCell.classes()).toContain('down')
    expect(priceCell.classes()).not.toContain('up')

    const changeBadge = wrapper.find('.change-badge')
    expect(changeBadge.classes()).toContain('down')
    expect(changeBadge.classes()).not.toContain('up')
  })

  // -- Ticker data → high / low display -------------------------------------

  it('shows 24h high and low values when ticker data is available', () => {
    setTickerData({
      'BTC-USDT': makeTicker({
        high24h: '51000.00',
        low24h: '47000.00',
      }),
    })

    const wrapper = mountPanel({ symbols: ['BTC-USDT'] })

    const highLowValues = wrapper.findAll('.high-low-row .value')
    expect(highLowValues[0].text()).not.toBe('-')
    expect(highLowValues[1].text()).not.toBe('-')
  })

  // -- Price formatting -----------------------------------------------------

  it('formats price with correct decimal precision from ticker data', () => {
    setTickerData({
      'BTC-USDT': makeTicker({ last: '50000.1234' }),
    })

    const wrapper = mountPanel({ symbols: ['BTC-USDT'] })

    // getPricePrecision detects 4 decimal places from the raw string
    expect(wrapper.find('.ticker-row .col-price').text()).toBe('50000.1234')
  })

  it('formats change percentage with two decimal places and sign', () => {
    setTickerData({
      'BTC-USDT': makeTicker({ last: '49000.00', open24h: '48000.00' }),
    })

    const wrapper = mountPanel({ symbols: ['BTC-USDT'] })

    // (49000 - 48000) / 48000 * 100 ≈ 2.0833... → "+2.08%"
    expect(wrapper.find('.change-badge').text()).toBe('+2.08%')
  })
})
