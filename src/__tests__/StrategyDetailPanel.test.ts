import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import StrategyDetailPanel from '@/components/strategy/StrategyDetailPanel.vue'

// Register mock element-plus components globally via mount options
const mockComponents = {
  'el-card': {
    template: '<div class="el-card"><div class="el-card__header"><slot name="header" /></div><div><slot /></div><slot name="footer" /></div>',
    props: ['shadow'],
  },
  'el-icon': {
    template: '<i class="el-icon"><slot/></i>',
  },
  'el-tag': {
    template: '<span class="el-tag" :class="type"><slot/></span>',
    props: ['type', 'size', 'effect'],
  },
  'el-descriptions': {
    template: '<div class="el-descriptions"><slot/></div>',
    props: ['column', 'border'],
  },
  'el-descriptions-item': {
    template: '<div class="el-descriptions-item"><span class="label">{{ label }}</span><slot/></div>',
    props: ['label', 'span'],
  },
  'el-button': {
    template: '<button class="el-button" :class="[type, size]" :disabled="disabled"><slot/></button>',
    props: ['type', 'size', 'disabled', 'round', 'circle', 'plain'],
  },
  'el-switch': {
    template: '<div class="el-switch"><slot/></div>',
    props: ['modelValue', 'disabled', 'size'],
  },
  Operation: { template: '<span class="mock-icon" />' },
  Top: { template: '<span class="mock-icon" />' },
  Bottom: { template: '<span class="mock-icon" />' },
}

describe('StrategyDetailPanel', () => {
  const defaults = { global: { components: mockComponents } }

  it('renders default description when not provided', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active' },
      ...defaults,
    })
    expect(wrapper.text()).toContain('暂无策略描述')
  })

  it('renders with empty tags/symbols/metrics arrays by default', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active' },
      ...defaults,
    })
    expect(wrapper.text()).toContain('s1')
    expect(wrapper.text()).toContain('暂无策略描述')
  })

  it('renders strategyId and status text', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 'test-123', status: 'active' },
      ...defaults,
    })
    expect(wrapper.text()).toContain('test-123')
    expect(wrapper.text()).toContain('运行中')
  })

  it('renders all 6 status texts correctly', () => {
    const cases: Array<{ status: 'active' | 'inactive' | 'pending' | 'error' | 'warning' | 'draft'; text: string }> = [
      { status: 'active', text: '运行中' },
      { status: 'inactive', text: '已停止' },
      { status: 'pending', text: '待运行' },
      { status: 'error', text: '运行异常' },
      { status: 'warning', text: '预警' },
      { status: 'draft', text: '草稿' },
    ]
    for (const { status, text } of cases) {
      const wrapper = mount(StrategyDetailPanel, {
        props: { strategyId: 's1', status },
        ...defaults,
      })
      expect(wrapper.text()).toContain(text)
    }
  })

  it('renders custom description', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active', description: 'A custom description text' },
      ...defaults,
    })
    expect(wrapper.text()).toContain('A custom description text')
  })

  it('renders tags from props', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active', tags: ['趋势策略', '高频交易'] },
      ...defaults,
    })
    expect(wrapper.text()).toContain('趋势策略')
    expect(wrapper.text()).toContain('高频交易')
  })

  it('renders symbols from props', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: {
        strategyId: 's1', status: 'active',
        symbols: [
          '000001.SZ',
          '600519.SH',
        ],
      },
      ...defaults,
    })
    expect(wrapper.text()).toContain('000001')
    expect(wrapper.text()).toContain('600519')
  })

  it('renders strategyType and isRunning props', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active', strategyType: 'TrendFollowing', isRunning: true },
      ...defaults,
    })
    expect(wrapper.text()).toContain('TrendFollowing')
  })

  // ── formatDate ──

  it('formatDate shows with default timestamp', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active' },
      ...defaults,
    })
    expect(wrapper.text()).toContain('创建时间')
  })

  it('formatDate handles zero timestamp', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active', createTime: 0 },
      ...defaults,
    })
    expect(wrapper.text()).toContain('1970')
  })

  it('formatDate handles large timestamp', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active', createTime: 1735689600000 },
      ...defaults,
    })
    expect(wrapper.text()).toContain('2025')
  })

  // ── Footer buttons ──

  it('emits edit event when edit button clicked', async () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active' },
      ...defaults,
    })
    const btn = wrapper.findAll('button').find((b) => b.text().includes('编辑策略'))
    expect(btn).toBeDefined()
    await btn!.trigger('click')
    expect(wrapper.emitted('edit')).toBeTruthy()
  })

  it('emits start event when start button clicked', async () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'inactive', isRunning: false },
      ...defaults,
    })
    const btn = wrapper.findAll('button').find((b) => b.text().includes('启动'))
    expect(btn).toBeDefined()
    await btn!.trigger('click')
    expect(wrapper.emitted('start')).toBeTruthy()
  })

  it('emits stop event when stop button clicked', async () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active', isRunning: true },
      ...defaults,
    })
    const btn = wrapper.findAll('button').find((b) => b.text().includes('停止'))
    expect(btn).toBeDefined()
    await btn!.trigger('click')
    expect(wrapper.emitted('stop')).toBeTruthy()
  })

  it('emits refresh event when button clicked', async () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active' },
      ...defaults,
    })
    const btn = wrapper.findAll('button').find((b) => b.text().includes('刷新'))
    expect(btn).toBeDefined()
    await btn!.trigger('click')
    expect(wrapper.emitted('refresh')).toBeTruthy()
  })

  // ── Start/Stop disabled state ──

  it('start disabled and stop enabled when isRunning=true', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'active', isRunning: true },
      ...defaults,
    })
    const startBtn = wrapper.findAll('button').find((b) => b.text().includes('启动'))
    const stopBtn = wrapper.findAll('button').find((b) => b.text().includes('停止'))
    expect(startBtn?.attributes('disabled')).toBeDefined()
    expect(stopBtn?.attributes('disabled')).toBeUndefined()
  })

  it('start enabled and stop disabled when isRunning=false', () => {
    const wrapper = mount(StrategyDetailPanel, {
      props: { strategyId: 's1', status: 'inactive', isRunning: false },
      ...defaults,
    })
    const startBtn = wrapper.findAll('button').find((b) => b.text().includes('启动'))
    const stopBtn = wrapper.findAll('button').find((b) => b.text().includes('停止'))
    expect(startBtn?.attributes('disabled')).toBeUndefined()
    expect(stopBtn?.attributes('disabled')).toBeDefined()
  })
})
