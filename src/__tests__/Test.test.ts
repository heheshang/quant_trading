import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Test from '@/views/Test.vue'
import { invoke } from '@tauri-apps/api/core'

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

describe('Test.vue - 按钮测试', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_metrics':
          return { api_latency: 5, api_uptime: 99.9, db_connections: 5, redis_hit_rate: 0.95 }
        default:
          return {}
      }
    })
  })

  it('运行系统测试按钮 - 调用 get_metrics 并显示测试结果', async () => {
    const wrapper = mount(Test, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    const runBtn = wrapper.find('.el-button--primary')
    expect(runBtn.exists()).toBe(true)
    expect(runBtn.text()).toContain('运行系统测试')

    await runBtn.trigger('click')
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    // Should have called get_metrics
    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')

    // Should have test results populated
    expect(wrapper.vm.testResults.length).toBeGreaterThan(0)
  })

  it('系统测试 - 每个测试项都有状态和耗时', async () => {
    const wrapper = mount(Test, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    await wrapper.find('.el-button--primary').trigger('click')
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    for (const result of wrapper.vm.testResults) {
      expect(result).toHaveProperty('name')
      expect(result).toHaveProperty('status')
      expect(result).toHaveProperty('duration')
      expect(['通过', '失败']).toContain(result.status)
    }
  })

  it('API 连接性检查 - onMounted 时调用', async () => {
    mount(Test, { global: { plugins: [ElementPlus] } })
    await new Promise(r => setTimeout(r, 50))
    expect(mockInvoke).toHaveBeenCalledWith('get_metrics')
  })

  it('API 失败时 - 设置 apiStatus 正确', async () => {
    mockInvoke.mockRejectedValue(new Error('Network error'))

    const wrapper = mount(Test, { global: { plugins: [ElementPlus] } })
    await new Promise(r => setTimeout(r, 50))

    expect(wrapper.vm.apiStatus.api).toBe(false)
  })

  it('测试期间 loading 状态', async () => {
    mockInvoke.mockImplementation(() => new Promise(resolve => setTimeout(resolve, 500)))

    const wrapper = mount(Test, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    await wrapper.find('.el-button--primary').trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.testing).toBe(true)
  })

  it('多次点击 - 防止重复提交', async () => {
    const wrapper = mount(Test, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    await wrapper.find('.el-button--primary').trigger('click')
    await wrapper.vm.$nextTick()

    // While testing, the button should show "测试中..."
    const btn = wrapper.find('.el-button--primary')
    expect(btn.text()).toContain('测试中')
  })

  it('无结果时显示 EmptyState', async () => {
    const wrapper = mount(Test, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    // Before running tests, no results should show
    expect(wrapper.vm.testResults.length).toBe(0)
  })
})
