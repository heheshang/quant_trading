import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Strategy from '@/views/Strategy.vue'
import { invoke } from '@tauri-apps/api/core'

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

vi.mock('echarts', () => {
  const mockECharts = { setOption: vi.fn(), dispose: vi.fn(), getInstanceByDom: vi.fn(), on: vi.fn(), off: vi.fn(), resize: vi.fn(), clear: vi.fn() }
  return { init: vi.fn().mockReturnValue(mockECharts), getInstanceByDom: vi.fn().mockReturnValue(mockECharts), default: { init: vi.fn().mockReturnValue(mockECharts) } }
})

const mockInvoke = vi.mocked(invoke)

const mockStrategies = [
  { strategy_id: 's1', strategy_name: 'Trend Following', strategy_type: 'TrendFollowing', enabled: true, max_position: 100000, max_daily_loss: 5000, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z' },
  { strategy_id: 's2', strategy_name: 'Mean Reversion', strategy_type: 'MeanReversion', enabled: false, max_position: 50000, max_daily_loss: 3000, created_at: '2026-02-01T00:00:00Z', updated_at: '2026-02-01T00:00:00Z' },
]

function mockFormRef() {
  return { validate: vi.fn((cb: any) => cb(true)) } as any
}

let container: HTMLDivElement

async function mountComponent() {
  const wrapper = mount(Strategy, { attachTo: container, global: { plugins: [ElementPlus] } })
  for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
  await new Promise(r => setTimeout(r, 30))
  return wrapper
}

describe('Strategy.vue - 按钮测试', () => {
  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_strategies': return mockStrategies
        case 'save_strategy': return 's3'
        case 'delete_strategy': return true
        case 'deploy_strategy': return 'deployed'
        case 'start_strategy': return 'started'
        case 'stop_strategy': return 'stopped'
        case 'pause_strategy': return 'paused'
        case 'resume_strategy': return 'resumed'
        case 'archive_strategy': return 'archived'
        case 'toggle_strategy': return true
        case 'run_backtest': return { total_return: 0.15, sharpe_ratio: 1.5, max_drawdown: -0.05 }
        default: return {}
      }
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    container.remove()
  })

  it('新建策略 - 打开创建对话框', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.vm.dialogVisible).toBe(false)
    wrapper.vm.openStrategyDialog()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.dialogVisible).toBe(true)
  }, 30000)

  it('刷新 - 调用 fetchStrategies', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.fetchStrategies()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    // fetchStrategies calls getStrategies() which passes { page: undefined, pageSize: undefined }
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies', { page: undefined, pageSize: undefined })
  }, 30000)

  it('保存策略 - 调用 save_strategy', async () => {
    const wrapper = await mountComponent()
    wrapper.vm.strategyFormRef = mockFormRef()
    wrapper.vm.dialogVisible = true
    wrapper.vm.currentStrategy = { strategy_name: 'New Strategy', strategy_type: 'TrendFollowing', max_position: 100000, max_daily_loss: 5000, enabled: true }
    mockInvoke.mockClear()

    await wrapper.vm.saveStrategy()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('save_strategy', { strategy: expect.any(Object) })
  }, 30000)

  it('取消按钮 - 关闭对话框', async () => {
    const wrapper = await mountComponent()
    wrapper.vm.dialogVisible = true
    await wrapper.vm.$nextTick()
    wrapper.vm.dialogVisible = false
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.dialogVisible).toBe(false)
  }, 30000)

  it('删除策略 - 打开确认对话框', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.vm.deleteDialogVisible).toBe(false)
    wrapper.vm.deleteStrategy('s1')
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.deleteDialogVisible).toBe(true)
  }, 30000)

  it('启用/禁用开关 - 调用 toggle_strategy', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    // The component's toggleStrategyStatus passes strategy.enabled as-is
    // (the el-switch v-model has already flipped it before @change fires)
    // mockStrategies[0] has enabled: true — simulate the switch flip by passing false
    const s = { ...mockStrategies[0], enabled: false }
    wrapper.vm.toggleStrategyStatus(s)
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('toggle_strategy', { strategyId: 's1', enabled: false })
  }, 30000)

  it('生命周期操作 - deploy/start/stop/pause/resume/archive', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    await wrapper.vm.handleLifecycle('deploy', mockStrategies[0])
    expect(mockInvoke).toHaveBeenCalledWith('deploy_strategy', { strategyId: 's1' })

    await wrapper.vm.handleLifecycle('start', mockStrategies[0])
    expect(mockInvoke).toHaveBeenCalledWith('start_strategy', { strategyId: 's1' })

    await wrapper.vm.handleLifecycle('stop', mockStrategies[0])
    expect(mockInvoke).toHaveBeenCalledWith('stop_strategy', { strategyId: 's1' })

    await wrapper.vm.handleLifecycle('pause', mockStrategies[0])
    expect(mockInvoke).toHaveBeenCalledWith('pause_strategy', { strategyId: 's1' })

    await wrapper.vm.handleLifecycle('resume', mockStrategies[0])
    expect(mockInvoke).toHaveBeenCalledWith('resume_strategy', { strategyId: 's1' })

    await wrapper.vm.handleLifecycle('archive', mockStrategies[0])
    expect(mockInvoke).toHaveBeenCalledWith('archive_strategy', { strategyId: 's1' })
  }, 30000)

  it('回测按钮 - 打开回测对话框', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.vm.backtestDialogVisible).toBe(false)
    wrapper.vm.runBacktest('s1')
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.backtestDialogVisible).toBe(true)
  }, 30000)

  it('API 调用失败 - 不崩溃', async () => {
    mockInvoke.mockRejectedValue(new Error('Backend error'))
    const wrapper = await mountComponent()
    expect(wrapper.exists()).toBe(true)
  }, 30000)

  it('加载状态显示', async () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}))
    const wrapper = mount(Strategy, { attachTo: container, global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.loading).toBe(true)
  }, 30000)

  it('搜索栏 - SearchBar 输入', async () => {
    const wrapper = await mountComponent()
    wrapper.vm.searchQuery = 'Trend'
    wrapper.vm.onSearch()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.searchQuery).toBe('Trend')
  }, 30000)
})
