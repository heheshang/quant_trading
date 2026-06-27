import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Backtest from '@/views/Backtest.vue'
import { invoke } from '@tauri-apps/api/core'

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

const mockResult = { total_return: 0.156, annual_return: 0.12, sharpe_ratio: 1.8, max_drawdown: -0.05, win_rate: 0.65, total_trades: 120, initial_capital: 1000000, final_capital: 1156000 }

let container: HTMLDivElement

async function mountComponent() {
  const wrapper = mount(Backtest, { attachTo: container, global: { plugins: [ElementPlus] } })
  await wrapper.vm.$nextTick()
  await wrapper.vm.$nextTick()
  await new Promise(r => setTimeout(r, 30))
  return wrapper as unknown as { vm: Record<string, any>; exists: () => boolean }
}

describe('Backtest.vue - 按钮测试', () => {
  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_strategies': return [{ strategy_id: 's1', strategy_name: 'Test Strategy', strategy_type: 'TrendFollowing', enabled: true }]
        case 'run_backtest': return mockResult
        case 'get_backtest_results': return [{ id: 1, strategy_name: 'S1', start_date: '2026-01-01', end_date: '2026-06-01', total_return: 0.1, annual_return: 0.08, sharpe_ratio: 1.5, max_drawdown: -0.03 }]
        case 'get_backtest_result': return mockResult
        case 'delete_backtest_result': return true
        default: return {}
      }
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    container.remove()
  })

  it('开始回测 - 调用 run_backtest', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.backtestConfig = { strategyId: 's1', strategyName: 'Test Strategy', startDate: '2026-01-01', endDate: '2026-06-01', initialCapital: 1000000, commissionRate: 0.0003, slippage: 0.0001, symbols: 'BTC-USDT' }
    wrapper.vm.backtestFormRef = { validate: vi.fn().mockResolvedValue(undefined) } as any

    await wrapper.vm.runBacktest()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('run_backtest', {
      strategyId: 's1', startDate: '2026-01-01', endDate: '2026-06-01', initialCapital: 1000000, commissionRate: 0.0003, slippage: 0.0001, symbols: ['BTC-USDT'],
    })
  }, 30000)

  it('重置按钮 - 恢复配置默认值', async () => {
    const wrapper = await mountComponent()
    wrapper.vm.backtestConfig.initialCapital = 999999
    wrapper.vm.resetConfig()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.backtestConfig.initialCapital).not.toBe(999999)
  }, 30000)

  it('对比按钮 - 切换对比模式', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.vm.compareMode).toBe(false)
    wrapper.vm.compareMode = true
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.compareMode).toBe(true)
  }, 30000)

  it('导出 CSV - 触发下载', async () => {
    const wrapper = await mountComponent()
    const createObjectURL = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:test')
    const clickSpy = vi.fn()
    vi.spyOn(document, 'createElement').mockReturnValue({ click: clickSpy, href: '', download: '' } as any)

    wrapper.vm.exportHistoryCSV()
    expect(clickSpy).toHaveBeenCalled()
    clickSpy.mockRestore()
    createObjectURL.mockRestore()
  }, 30000)

  it('刷新历史 - 调用 fetchHistory', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()
    wrapper.vm.fetchHistory()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    expect(mockInvoke).toHaveBeenCalledWith('get_backtest_results', { limit: 50, offset: 0 })
  }, 30000)

  it('回测运行期间 loading 状态', async () => {
    const wrapper = await mountComponent()
    wrapper.vm.backtestFormRef = { validate: vi.fn().mockResolvedValue(undefined) } as any
    wrapper.vm.backtestConfig = { strategyId: 's1', strategyName: 'Test', startDate: '2026-01-01', endDate: '2026-06-01', initialCapital: 1000000, commissionRate: 0.0003, slippage: 0.0001, symbols: 'BTC-USDT' }
    mockInvoke.mockImplementation(() => new Promise(() => {}))

    wrapper.vm.runBacktest()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.running).toBe(true)
  }, 30000)

  it('API 调用失败 - 不崩溃', async () => {
    mockInvoke.mockRejectedValue(new Error('Backend error'))
    const wrapper = await mountComponent()
    expect(wrapper.exists()).toBe(true)
  }, 30000)
})
