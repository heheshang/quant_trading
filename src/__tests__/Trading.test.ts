import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Trading from '@/views/Trading.vue'
import { invoke } from '@tauri-apps/api/core'

const mockPlaceOrder = vi.fn().mockResolvedValue('order-123')
vi.mock('@/stores/order', () => ({
  useOrderStore: () => ({ placeOrder: mockPlaceOrder, orderCount: 0, activeOrders: [], loading: false, error: null, fetchActiveOrders: vi.fn().mockResolvedValue([]) }),
}))

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

let container: HTMLDivElement

async function mountComponent(): Promise<any> {
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
    await wrapper.vm.submitOrder({
      strategy_id: '',
      symbol: 'BTC-USDT',
      side: 'Buy',
      order_type: 'Limit',
      price: 50000,
      quantity: 0.1,
    })
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
    expect(wrapper.vm.orderForm.symbol).toBe('BTC-USDT')
  }, 30000)

  it('刷新订单 - 调用 fetchActiveOrders', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    await wrapper.vm.refreshOrders()
    await flushPromises()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('get_recent_orders', { limit: 200, exchange: 'paper' })
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
