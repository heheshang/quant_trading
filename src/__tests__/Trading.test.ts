import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Trading from '@/views/Trading.vue'
import { invoke } from '@tauri-apps/api/core'

vi.mock('@/composables/useWebSocketStatus', () => ({
  useWebSocketStatus: () => ({ status: 'connected', retryIn: 0, startListening: vi.fn(), cleanup: vi.fn() }),
}))

vi.mock('@/composables/useMarketData', () => ({
  useMarketData: () => ({ startListening: vi.fn(), cleanup: vi.fn(), tickerData: { value: {} }, trades: { value: {} }, orderbook: { value: {} }, candleData: { value: {} } }),
}))

const mockPlaceOrder = vi.fn().mockResolvedValue('order-123')
vi.mock('@/stores/order', () => ({
  useOrderStore: () => ({ placeOrder: mockPlaceOrder, orderCount: 0, activeOrders: [], loading: false, error: null, fetchActiveOrders: vi.fn().mockResolvedValue([]) }),
}))

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

function mockFormRef() {
  return { validate: vi.fn((cb: any) => cb(true)) } as any
}

let container: HTMLDivElement

async function mountComponent() {
  const wrapper = mount(Trading, { attachTo: container, global: { plugins: [ElementPlus] } })
  for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
  await new Promise(r => setTimeout(r, 30))
  return wrapper
}

describe('Trading.vue - 按钮测试', () => {
  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_account_info': return { total_assets: 1000000, available_cash: 500000, market_value: 500000, daily_pnl: 5000 }
        case 'get_positions': return []
        case 'get_active_orders': return []
        case 'get_strategies': return []
        case 'check_okx_status': return { connected: true, demo_trading: true }
        case 'get_okx_instruments': return [{ instId: 'BTC-USDT', baseCcy: 'BTC', quoteCcy: 'USDT', instType: 'SPOT' }]
        case 'get_okx_balance': return [{ ccy: 'BTC', balance: 0.5, frozen: 0, available: 0.5 }]
        case 'get_okx_positions': return [{ instId: 'BTC-USDT', pos: 0.1, avgPx: 50000, upl: 100 }]
        case 'get_okx_announcements': return []
        default: return {}
      }
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    container.remove()
  })

  it('提交订单 - 调用 placeOrder', async () => {
    const wrapper = await mountComponent()
    wrapper.vm.orderFormRef = mockFormRef()
    wrapper.vm.orderForm.symbol = 'BTC-USDT'
    wrapper.vm.orderForm.side = 'Buy'
    wrapper.vm.orderForm.order_type = 'Limit'
    wrapper.vm.orderForm.price = 50000
    wrapper.vm.orderForm.quantity = 0.1

    await wrapper.vm.submitOrder()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockPlaceOrder).toHaveBeenCalled()
  }, 30000)

  it('重置按钮 - 清空表单', async () => {
    const wrapper = await mountComponent()
    wrapper.vm.orderForm.symbol = 'BTC-USDT'
    wrapper.vm.resetOrderForm()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.orderForm.symbol).toBe('600519.SH')
  }, 30000)

  it('刷新订单 - 调用 fetchActiveOrders', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.refreshOrders()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('get_active_orders')
  }, 30000)

  it('导出 CSV - 触发浏览器下载', async () => {
    const wrapper = await mountComponent()
    const createObjectURL = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:test')
    const clickSpy = vi.fn()
    vi.spyOn(document, 'createElement').mockReturnValue({ href: '', click: clickSpy, download: '' } as any)

    wrapper.vm.exportOrdersCSV()
    expect(clickSpy).toHaveBeenCalled()

    clickSpy.mockRestore()
    createObjectURL.mockRestore()
  }, 30000)

  it('OKX 刷新状态 - 调用 check_okx_status', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.fetchOkxStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('check_okx_status')
  }, 30000)

  it('OKX 刷新余额 - 调用 get_okx_balance', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.fetchOkxBalance()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('get_okx_balance', { ccy: undefined })
  }, 30000)

  it('OKX 刷新持仓 - 调用 get_okx_positions', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.fetchOkxPositions()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('get_okx_positions', { instId: undefined })
  }, 30000)

  it('API 调用失败 - 不崩溃', async () => {
    mockInvoke.mockRejectedValue(new Error('Network error'))
    const wrapper = await mountComponent()
    expect(wrapper.exists()).toBe(true)
  }, 30000)

  it('撤单确认对话框 - 打开/关闭', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.vm.cancelDialogVisible).toBe(false)
    wrapper.vm.cancelOrder(123)
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.cancelDialogVisible).toBe(true)
    expect(wrapper.vm.orderIdToCancel).toBe(123)
  }, 30000)
})
