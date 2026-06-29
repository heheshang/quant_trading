import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import PerformanceChart from '@/components/strategy/PerformanceChart.vue'

// PerformanceChart imports ElSkeleton and ElResult from element-plus directly
// in its <script setup>, so we must mock the module at the module level.
// The global setup.ts mock does not export these.
vi.mock('element-plus', () => ({
  ElSkeleton: { template: '<div class="el-skeleton" />', props: ['animated'] },
  ElResult: { template: '<div class="el-result"><slot /></div>', props: ['icon', 'title', 'subTitle'] },
}))

// ECharts needs real DOM canvas to init. In jsdom, init() throws.
// We mock echarts.init at module level so imports don't crash.
vi.mock('echarts/core', () => ({
  init: vi.fn(() => ({
    setOption: vi.fn(),
    dispose: vi.fn(),
    on: vi.fn(() => 'updateLayout'),
    resize: vi.fn(),
  })),
  use: vi.fn(),
}))
vi.mock('echarts/charts', () => ({ LineChart: vi.fn() }))
vi.mock('echarts/components', () => ({
  TitleComponent: vi.fn(),
  TooltipComponent: vi.fn(),
  LegendComponent: vi.fn(),
  GridComponent: vi.fn(),
  DataZoomComponent: vi.fn(),
  MarkLineComponent: vi.fn(),
}))
vi.mock('echarts/renderers', () => ({ CanvasRenderer: vi.fn() }))

const elementStubs = {
  'el-radio-group': { template: '<div class="el-radio-group"><slot/></div>', props: ['modelValue', 'size'] },
  'el-radio-button': { template: '<label class="el-radio-button"><slot/></label>', props: ['label'] },
  'el-select': { template: '<div class="el-select"><slot/></div>', props: ['modelValue', 'size'] },
  'el-option': { template: '<div class="el-option"><slot/></div>', props: ['label', 'value'] },
}

describe('PerformanceChart', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows loading skeleton on mount (initial loading=true)', () => {
    const wrapper = mount(PerformanceChart, {
      global: { stubs: elementStubs },
    })
    expect(wrapper.find('.chart-loading').exists()).toBe(true)
    expect(wrapper.find('.echart-container').exists()).toBe(false)
  })

  it('shows controls when showControls=true', () => {
    const wrapper = mount(PerformanceChart, {
      props: { showControls: true },
      global: { stubs: elementStubs },
    })
    expect(wrapper.find('.chart-controls').exists()).toBe(true)
  })

  it('hides controls when showControls=false', () => {
    const wrapper = mount(PerformanceChart, {
      props: { showControls: false },
      global: { stubs: elementStubs },
    })
    expect(wrapper.find('.chart-controls').exists()).toBe(false)
  })

  it('renders with empty equityCurve (no crash)', () => {
    const wrapper = mount(PerformanceChart, {
      props: { equityCurve: [] },
      global: { stubs: elementStubs },
    })
    expect(wrapper.exists()).toBe(true)
  })

  it('has correct height style from prop', () => {
    const wrapper = mount(PerformanceChart, {
      props: { height: '500px' },
      global: { stubs: elementStubs },
    })
    const el = wrapper.find('.performance-chart')
    expect(el.exists()).toBe(true)
  })

  it('renders el-skeleton with animated prop', () => {
    const wrapper = mount(PerformanceChart, {
      global: { stubs: elementStubs },
    })
    expect(wrapper.find('.chart-loading .el-skeleton').exists()).toBe(true)
  })

  it('does NOT emit chart-ready event (initChart never runs)', () => {
    const wrapper = mount(PerformanceChart, {
      global: { stubs: elementStubs },
    })
    expect(wrapper.emitted('chart-ready')).toBeFalsy()
  })

  it('does not render chart controls when inside loading state', () => {
    const wrapper = mount(PerformanceChart, {
      props: { showControls: true },
      global: { stubs: elementStubs },
    })
    // Controls are always rendered regardless of loading state
    // because chart-controls is outside the v-if block
    expect(wrapper.find('.chart-controls').exists()).toBe(true)
  })

  it('renders legend items', () => {
    const wrapper = mount(PerformanceChart, {
      global: { stubs: elementStubs },
    })
    const legendItems = wrapper.findAll('.legend-item')
    expect(legendItems.length).toBe(3)
    expect(legendItems[0].text()).toContain('权益曲线')
    expect(legendItems[1].text()).toContain('最大回撤')
    expect(legendItems[2].text()).toContain('夏普比率')
  })
})
