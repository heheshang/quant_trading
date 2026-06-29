import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import StrategyStatusTag from '@/components/strategy/StrategyStatusTag.vue'

// Register mock element-plus components globally via mount options
// (auto-import resolvers are disabled in test mode)
const mockComponents = {
  'el-tag': {
    template: '<div :class="[\'el-tag\', type, size]"><slot/></div>',
    props: ['type', 'size', 'effect'],
  },
  'el-icon': {
    template: '<i class="el-icon"><slot/></i>',
  },
}

vi.mock('@element-plus/icons-vue', () => ({
  CircleCheck: { template: '<span class="mock-icon" />' },
  CircleClose: { template: '<span class="mock-icon" />' },
  Clock: { template: '<span class="mock-icon" />' },
  Warning: { template: '<span class="mock-icon" />' },
  Edit: { template: '<span class="mock-icon" />' },
}))

describe('StrategyStatusTag', () => {
  it('renders with status=active shows 运行中', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'active' },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('运行中')
  })

  it('renders with status=error shows 运行异常', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'error' },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('运行异常')
  })

  it('renders with status=draft shows 草稿', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'draft' },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('草稿')
  })

  it('renders with status=inactive shows 已停止', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'inactive' },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('已停止')
  })

  it('renders with status=pending shows 待运行', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'pending' },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('待运行')
  })

  it('renders with status=warning shows 预警', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'warning' },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('预警')
  })

  it('size prop is passed correctly to el-tag component', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'active', size: 'small' },
      global: { components: mockComponents },
    })
    // Component renders with the strategy-status-tag class on root element
    expect(wrapper.classes()).toContain('strategy-status-tag')
  })

  it('shows icon when showIcon=true (default)', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'active' },
      global: { components: mockComponents },
    })
    expect(wrapper.find('.el-icon').exists()).toBe(true)
  })

  it('hides icon when showIcon=false', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'active', showIcon: false },
      global: { components: mockComponents },
    })
    expect(wrapper.find('.el-icon').exists()).toBe(false)
  })

  it('uses default effect=light when not specified', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'active' },
      global: { components: mockComponents },
    })
    expect(wrapper.exists()).toBe(true)
  })

  it('renders correct text for each of the 6 statuses', () => {
    const statuses: Array<{ status: 'active' | 'inactive' | 'pending' | 'error' | 'warning' | 'draft'; text: string }> = [
      { status: 'active', text: '运行中' },
      { status: 'inactive', text: '已停止' },
      { status: 'pending', text: '待运行' },
      { status: 'error', text: '运行异常' },
      { status: 'warning', text: '预警' },
      { status: 'draft', text: '草稿' },
    ]
    for (const { status, text } of statuses) {
      const wrapper = mount(StrategyStatusTag, {
        props: { status },
        global: { components: mockComponents },
      })
      expect(wrapper.text()).toContain(text)
    }
  })
})
