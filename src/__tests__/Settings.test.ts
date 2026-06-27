import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Settings from '@/views/Settings.vue'
import { invoke } from '@tauri-apps/api/core'

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() }, ElNotification: { success: vi.fn(), error: vi.fn() }, ElMessageBox: { confirm: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

const defaultConfig = {
  database: { host: 'localhost', port: 5432, username: 'postgres', password: '', database: 'quant_trading', max_connections: 10 },
  redis: { host: 'localhost', port: 6379, password: '', db: 0, pool_size: 10 },
  trading: { enable_paper_trading: true, max_orders_per_second: 10, default_commission_rate: 0.0003, default_slippage: 0.0001, order_timeout_seconds: 30 },
  risk: { max_position_size: 0.2, max_daily_loss: 0.05, max_drawdown: 0.15, enable_pre_trade_check: true, enable_real_time_monitor: true, var_confidence_level: 0.95 },
  monitoring: { enable_prometheus: true, prometheus_port: 9090, log_level: 'info', alert_email: '', alert_webhook: '' },
  security: { enable_encryption: true, enable_2fa: false, jwt_secret: 'secret', token_expiry_hours: 24, allowed_ips: [] },
}

function mockFormRef() {
  return { validate: vi.fn().mockResolvedValue(undefined) } as any
}

let container: HTMLDivElement

async function mountComponent(): Promise<any> {
  const wrapper = mount(Settings, { attachTo: container, global: { plugins: [ElementPlus] } })
  for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
  await new Promise(r => setTimeout(r, 30))
  return wrapper
}

describe('Settings.vue - 按钮测试', () => {
  beforeEach(() => {
    container = document.createElement('div')
    document.body.appendChild(container)
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_config': return defaultConfig
        case 'update_config': return true
        case 'check_okx_status': return { connected: true, demo_trading: true, exchange_time: '2026-06-26T00:00:00Z', message: 'OK' }
        default: return {}
      }
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    container.remove()
  })

  it('保存配置 - 调用 update_config', async () => {
    const wrapper = await mountComponent()
    const formRefs = ['systemInfoFormRef', 'databaseFormRef', 'redisFormRef', 'tradingFormRef', 'riskFormRef', 'monitoringFormRef', 'securityFormRef']
    formRefs.forEach(name => { (wrapper.vm as any)[name] = mockFormRef() })
    mockInvoke.mockClear()

    await wrapper.vm.saveConfig()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('update_config', { config: expect.any(Object) })
  }, 30000)

  it('重置按钮 - 打开确认对话框', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.vm.resetDialogVisible).toBe(false)
    wrapper.vm.resetConfig()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.resetDialogVisible).toBe(true)
  }, 30000)

  it('导出配置 - 触发 JSON 下载', async () => {
    const wrapper = await mountComponent()
    const createObjectURL = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:test')
    const clickSpy = vi.fn()
    vi.spyOn(document, 'createElement').mockReturnValue({ click: clickSpy, href: '', download: '' } as any)

    wrapper.vm.exportConfig()
    expect(clickSpy).toHaveBeenCalled()
    clickSpy.mockRestore()
    createObjectURL.mockRestore()
  }, 30000)

  it('检测 OKX 连接 - 调用 check_okx_status', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.fetchOkxConnStatus()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('check_okx_status')
  }, 30000)

  it('切换 Tab - activeTab 更新', async () => {
    const wrapper = await mountComponent()
    expect(wrapper.vm.activeTab).toBe('basic')
    wrapper.vm.activeTab = 'database'; await wrapper.vm.$nextTick(); expect(wrapper.vm.activeTab).toBe('database')
    wrapper.vm.activeTab = 'redis'; await wrapper.vm.$nextTick(); expect(wrapper.vm.activeTab).toBe('redis')
    wrapper.vm.activeTab = 'risk'; await wrapper.vm.$nextTick(); expect(wrapper.vm.activeTab).toBe('risk')
  }, 30000)

  it('保存配置失败 - 不崩溃', async () => {
    mockInvoke.mockRejectedValue(new Error('Save failed'))
    const wrapper = await mountComponent()
    const formRefs = ['systemInfoFormRef', 'databaseFormRef', 'redisFormRef', 'tradingFormRef', 'riskFormRef', 'monitoringFormRef', 'securityFormRef']
    formRefs.forEach(name => { (wrapper.vm as any)[name] = mockFormRef() })

    await wrapper.vm.saveConfig()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    expect(wrapper.exists()).toBe(true)
  }, 30000)

  it('loading 状态 - 保存期间', async () => {
    const wrapper = await mountComponent()
    const formRefs = ['systemInfoFormRef', 'databaseFormRef', 'redisFormRef', 'tradingFormRef', 'riskFormRef', 'monitoringFormRef', 'securityFormRef']
    formRefs.forEach(name => { (wrapper.vm as any)[name] = mockFormRef() })
    mockInvoke.mockImplementation(() => new Promise(() => {}))

    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'update_config') return new Promise(() => {})
      if (cmd === 'get_config') return defaultConfig
      return {}
    })
    wrapper.vm.saveConfig()
    await new Promise(r => setTimeout(r, 80))
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.saving).toBe(true)
  }, 30000)

  it('onMounted 调用 get_config', async () => {
    mount(Settings, { attachTo: container, global: { plugins: [ElementPlus] } })
    await new Promise(r => setTimeout(r, 50))
    expect(mockInvoke).toHaveBeenCalledWith('get_config')
  }, 30000)
})
