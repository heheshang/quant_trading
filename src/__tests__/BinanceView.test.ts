import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BinanceView from '@/views/Binance.vue'
import { invoke } from '@tauri-apps/api/core'
import type { BinanceOrder } from '@/services/types'

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

let container: HTMLDivElement

async function mountComponent(): Promise<any> {
  const wrapper = mount(BinanceView, {
    attachTo: container,
    global: { plugins: [ElementPlus] },
  })
  await flushPromises()
  await wrapper.vm.$nextTick()
  return wrapper
}

describe('BinanceView', () => {
  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_binance_balance':
          return [{ asset: 'USDT', free: 100.5, locked: 0 }]
        case 'get_binance_positions':
          return [
            {
              symbol: 'BTCUSDT',
              position_amt: 0.001,
              entry_price: 50000,
              mark_price: 51000,
              un_realized_profit: 1,
              liquidation_price: 0,
              leverage: '10',
              margin_type: 'crossed',
              notional: 50,
              position_side: 'BOTH',
            },
          ]
        case 'get_binance_orders':
          return []
        case 'get_binance_candles':
          return []
        case 'check_binance_status':
          return { connected: true }
        default:
          return {}
      }
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    container.remove()
  })

  it('renders positions from get_binance_positions', async () => {
    const wrapper = await mountComponent()
    const text = wrapper.text()
    expect(text).toContain('BTC-USDT')
    expect(text).toContain('0.001')
  })

  it('renders orders empty state when no orders', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.text()).toContain('暂无订单')
  })

  it('loads active orders by default and history on toggle', async () => {
    const wrapper = await mountComponent()
    expect(mockInvoke).toHaveBeenCalledWith('get_binance_orders', {
      symbol: 'BTCUSDT',
      history: false,
      limit: undefined,
    })
    // Simulate the radio group flipping to history before the @change handler.
    wrapper.vm.ordersHistory = true
    wrapper.vm.toggleHistory()
    await flushPromises()
    await wrapper.vm.$nextTick()
    expect(mockInvoke).toHaveBeenCalledWith('get_binance_orders', {
      symbol: 'BTCUSDT',
      history: true,
      limit: undefined,
    })
  })

  it('cancels an order and reloads orders', async () => {
    const wrapper = await mountComponent()
    const order: BinanceOrder = {
      symbol: 'BTCUSDT',
      order_id: 99,
      client_order_id: 'x',
      status: 'NEW',
      executed_qty: 0,
      cummulative_quote_qty: 0,
      price: 50000,
    }
    await wrapper.vm.cancelOrder(order)
    expect(mockInvoke).toHaveBeenCalledWith('cancel_binance_order', {
      symbol: 'BTCUSDT',
      orderId: 99,
    })
  })
})
