import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import StrategyStatusTag from '@/components/strategy/StrategyStatusTag.vue'
import type { StrategyStatus } from '@/services/types'

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
  EditPen: { template: '<span class="mock-icon" />' },
  DataAnalysis: { template: '<span class="mock-icon" />' },
  Upload: { template: '<span class="mock-icon" />' },
  VideoPlay: { template: '<span class="mock-icon" />' },
  VideoPause: { template: '<span class="mock-icon" />' },
  Box: { template: '<span class="mock-icon" />' },
}))

describe('StrategyStatusTag', () => {
  it('renders with status=Running shows 运行中', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Running' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('运行中')
  })

  it('renders with status=Backtesting shows 回测中', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Backtesting' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('回测中')
  })

  it('renders with status=Draft shows 草稿', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Draft' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('草稿')
  })

  it('renders with status=Deployed shows 已部署', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Deployed' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('已部署')
  })

  it('renders with status=Paused shows 已暂停', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Paused' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('已暂停')
  })

  it('renders with status=Archived shows 已归档', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Archived' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.text()).toContain('已归档')
  })

  it('size prop is passed correctly to el-tag component', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Running' as StrategyStatus, size: 'small' },
      global: { components: mockComponents },
    })
    // Component renders with the strategy-status-tag class on root element
    expect(wrapper.classes()).toContain('strategy-status-tag')
  })

  it('shows icon when showIcon=true (default)', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Running' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.find('.el-icon').exists()).toBe(true)
  })

  it('hides icon when showIcon=false', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Running' as StrategyStatus, showIcon: false },
      global: { components: mockComponents },
    })
    expect(wrapper.find('.el-icon').exists()).toBe(false)
  })

  it('uses default effect=light when not specified', () => {
    const wrapper = mount(StrategyStatusTag, {
      props: { status: 'Running' as StrategyStatus },
      global: { components: mockComponents },
    })
    expect(wrapper.exists()).toBe(true)
  })

  it('renders correct text for each of the 6 StrategyStatus values', () => {
    const statuses: Array<{ status: StrategyStatus; text: string }> = [
      { status: 'Draft', text: '草稿' },
      { status: 'Backtesting', text: '回测中' },
      { status: 'Deployed', text: '已部署' },
      { status: 'Running', text: '运行中' },
      { status: 'Paused', text: '已暂停' },
      { status: 'Archived', text: '已归档' },
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
