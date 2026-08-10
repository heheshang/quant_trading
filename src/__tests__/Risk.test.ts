import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Risk from '@/views/Risk.vue'
import PreTradeCheckForm from '@/components/risk/PreTradeCheckForm.vue'
import RiskAlertsTable from '@/components/risk/RiskAlertsTable.vue'
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

function mockFormRef() {
  return { validate: vi.fn((cb: any) => cb(true)) } as any
}

async function mountComponent(): Promise<any> {
  const wrapper = mount(Risk, { global: { plugins: [ElementPlus] } })
  for (let i = 0; i < 5; i++) await wrapper.vm.$nextTick()
  await new Promise(r => setTimeout(r, 30))
  return wrapper
}

describe('Risk.vue - 按钮测试', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_risk_metrics': return { var_95: 0.02, var_99: 0.04, max_position_size: 0.2, max_daily_loss: 0.05 }
        case 'get_risk_config': return { max_position_size: 0.2, max_daily_loss: 0.05, max_drawdown: 0.15, max_concentration: 0.2, enable_pre_trade_check: true, enable_real_time_monitor: true, var_confidence_level: 0.95 }
        case 'update_risk_config': return true
        case 'pre_trade_check': return true
        case 'get_alerts': return [{ alert_id: 1, level: 'Warning', source: 'Risk', message: 'Risk limit approaching', timestamp: new Date().toISOString(), acknowledged: false }, { alert_id: 2, level: 'Critical', source: 'System', message: 'Connection lost', timestamp: new Date().toISOString(), acknowledged: false }]
        case 'acknowledge_alert': return true
        default: return {}
      }
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('刷新指标 - 调用 get_risk_metrics', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    await wrapper.vm.fetchRiskMetrics()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('get_risk_metrics')
  })

  it('保存配置 - 调用 update_risk_config', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    await wrapper.vm.saveConfig({
      max_position_size: 0.2,
      max_daily_loss: 0.05,
      max_drawdown: 0.15,
      max_concentration: 0.2,
      enable_pre_trade_check: true,
      enable_real_time_monitor: true,
      var_confidence_level: 0.95,
    })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('update_risk_config', { config: expect.any(Object) })
  })

  it('保存配置 loading 状态', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockImplementation(() => new Promise(() => {}))

    wrapper.vm.saveConfig({
      max_position_size: 0.2,
      max_daily_loss: 0.05,
      max_drawdown: 0.15,
      max_concentration: 0.2,
      enable_pre_trade_check: true,
      enable_real_time_monitor: true,
      var_confidence_level: 0.95,
    })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    expect(wrapper.vm.saving).toBe(true)
  })

  it('风控检查 - 调用 pre_trade_check', async () => {
    const wrapper = await mountComponent()
    const preTrade = wrapper.findComponent(PreTradeCheckForm)
    preTrade.vm.formRef = mockFormRef()
    mockInvoke.mockClear()

    await preTrade.vm.runCheck()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('pre_trade_check', expect.any(Object))
  })

  it('重置风控测试 - 恢复默认值', async () => {
    const wrapper = await mountComponent()
    const preTrade = wrapper.findComponent(PreTradeCheckForm)
    preTrade.vm.testOrder.symbol = 'CHANGED'
    await wrapper.vm.$nextTick()
    preTrade.vm.resetForm()
    await wrapper.vm.$nextTick()
    expect(preTrade.vm.testOrder.symbol).toBe('600519.SH')
  })

  it('刷新告警 - 调用 get_alerts', async () => {
    const wrapper = await mountComponent()
    mockInvoke.mockClear()

    wrapper.vm.fetchRiskAlerts()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('get_alerts')
  })

  it('API 调用失败 - 不崩溃', async () => {
    mockInvoke.mockRejectedValue(new Error('Backend error'))
    const wrapper = await mountComponent()
    expect(wrapper.exists()).toBe(true)
  })

  it('告警级别筛选 - 更新 filteredAlerts', async () => {
    const wrapper = await mountComponent()
    const alertsTable = wrapper.findComponent(RiskAlertsTable)
    alertsTable.vm.levelFilter = 'Warning'
    await wrapper.vm.$nextTick()
    for (const alert of alertsTable.vm.filteredAlerts) {
      expect(['Warning', undefined, null]).toContain(alert.level)
    }
  })
})
