import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick, defineComponent, h } from 'vue'
import TradeStream from '@/components/dashboard/TradeStream.vue'
import { mockListen, mockUnlisten } from './setup'
import type { WsTrade } from '@/services/types'

// ---------------------------------------------------------------------------
// Stub element-plus components so we don't have to resolve .css files in jsdom.
// Each stub preserves the CSS class contract the real components emit, so we
// can assert against .el-table, .el-tag--success, .el-empty, etc.
// ---------------------------------------------------------------------------
const ElTableStub = defineComponent({
  name: 'ElTable',
  props: ['data', 'size', 'maxHeight', 'stripe'],
  emits: ['select', 'selectAll', 'selectionChange', 'cellMouseEnter', 'cellMouseLeave',
    'cellClick', 'cellDblclick', 'rowClick', 'rowContextmenu', 'rowDblclick',
    'headerClick', 'headerContextmenu', 'sortChange', 'filterChange',
    'currentChange', 'headerDragend', 'expandChange'],
  setup(props, { slots }) {
    return () => {
      const data = (props.data as Record<string, unknown>[]) ?? []
      const defaultSlot = slots.default
      if (!defaultSlot || data.length === 0) {
        return h('div', { class: 'el-table' })
      }
      const columnVNodes = defaultSlot()
      const columnSlots = (Array.isArray(columnVNodes) ? columnVNodes : [columnVNodes])
        .map((vnode) => {
          const vnodeChildren = vnode.children as
            | { default?: (scope: Record<string, unknown>) => unknown }
            | undefined
          return typeof vnodeChildren?.default === 'function'
            ? vnodeChildren.default
            : null
        })
        .filter(Boolean) as ((scope: Record<string, unknown>) => unknown)[]

      const rows = data.map((row: Record<string, unknown>) => {
        const cells = columnSlots.map((slotFn) =>
          h('td', { class: 'el-table__cell' }, [slotFn({ row }) as unknown as string]),
        )
        return h('tr', null, cells)
      })
      return h('div', { class: 'el-table' }, [
        h('div', { class: 'el-table__body-wrapper' }, [
          h('table', null, [h('tbody', null, rows)]),
        ]),
      ])
    }
  },
})

const ElTableColumnStub = defineComponent({
  name: 'ElTableColumn',
  props: ['label', 'prop', 'width', 'minWidth', 'align', 'sortable', 'fixed'],
  render() {
    return null
  },
})

const ElTagStub = defineComponent({
  name: 'ElTag',
  props: ['type', 'size', 'effect', 'hit', 'closable', 'disableTransitions'],
  setup(props, { slots }) {
    const typeClass = `el-tag--${props.type ?? 'info'}`
    return () => h('span', { class: ['el-tag', typeClass] }, slots.default?.())
  },
})

const ElEmptyStub = defineComponent({
  name: 'ElEmpty',
  props: ['description', 'image', 'imageSize'],
  setup(props, { slots }) {
    return () => {
      if (slots.description) return slots.description()
      return h('div', { class: 'el-empty' }, [
        h('div', { class: 'el-empty__description' }, props.description ?? ''),
      ])
    }
  },
})

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeTrade(overrides: Partial<WsTrade> = {}): WsTrade {
  return {
    inst_id: 'BTC-USDT',
    px: '50000',
    sz: '1.5',
    side: 'buy',
    ts: '2025-01-15T10:30:00Z',
    ...overrides,
  }
}

function getListenerCallback(): ((event: { payload: WsTrade[] }) => void) | null {
  const calls = mockListen.mock.calls
  const lastCall = calls[calls.length - 1]
  if (!lastCall) return null
  return lastCall[1] as (event: { payload: WsTrade[] }) => void
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('TradeStream', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockListen.mockResolvedValue(mockUnlisten)
  })

  async function mountTradeStream(symbol = 'BTC-USDT') {
    const wrapper = mount(TradeStream, {
      props: { symbol },
      global: {
        stubs: {
          ElTable: ElTableStub,
          ElTableColumn: ElTableColumnStub,
          ElTag: ElTagStub,
          ElEmpty: ElEmptyStub,
        },
      },
    })
    // Two ticks: one for the listen() promise, one for Vue reactivity flush.
    await nextTick()
    await nextTick()
    return wrapper
  }

  it('shows "等待成交数据..." el-empty when no trades received', async () => {
    const wrapper = await mountTradeStream()

    expect(wrapper.find('.el-empty').exists()).toBe(true)
    expect(wrapper.find('.el-empty__description').text()).toBe('等待成交数据...')
    expect(wrapper.find('.el-table').exists()).toBe(false)
  })

  it('renders el-table with trades data when trades arrive', async () => {
    const wrapper = await mountTradeStream()
    const callback = getListenerCallback()
    expect(callback).not.toBeNull()

    callback!({ payload: [makeTrade()] })
    await nextTick()

    expect(wrapper.find('.el-empty').exists()).toBe(false)
    expect(wrapper.find('.el-table').exists()).toBe(true)
    expect(wrapper.find('.count').text()).toBe('1 笔')
  })

  it('renders buy tag as success/green and sell tag as danger/red', async () => {
    const wrapper = await mountTradeStream()
    const callback = getListenerCallback()
    expect(callback).not.toBeNull()

    callback!({
      payload: [
        makeTrade({ side: 'buy', px: '50000' }),
        makeTrade({ side: 'sell', px: '51000' }),
      ],
    })
    await nextTick()

    const tags = wrapper.findAll('.el-tag')
    expect(tags).toHaveLength(2)

    // buy → el-tag--success (green in Western convention)
    expect(tags[0].classes()).toContain('el-tag--success')
    expect(tags[0].text()).toBe('买入')

    // sell → el-tag--danger (red in Western convention)
    expect(tags[1].classes()).toContain('el-tag--danger')
    expect(tags[1].text()).toBe('卖出')
  })

  it('truncates trades to 500 max', async () => {
    const wrapper = await mountTradeStream()
    const callback = getListenerCallback()
    expect(callback).not.toBeNull()

    const trades: WsTrade[] = Array.from({ length: 600 }, (_, i) =>
      makeTrade({
        px: String(i),
        ts: new Date(2025, 0, 15, 10, i % 60, 0).toISOString(),
      }),
    )
    callback!({ payload: trades })
    await nextTick()

    expect(wrapper.find('.count').text()).toBe('500 笔')
    const rows = wrapper.findAll('.el-table__body-wrapper tbody tr')
    expect(rows.length).toBe(500)
  })

  it('cleans up listener on unmount', async () => {
    const wrapper = await mountTradeStream()

    expect(mockListen).toHaveBeenCalledWith('ws:trades', expect.any(Function))

    wrapper.unmount()

    expect(mockUnlisten).toHaveBeenCalled()
  })
})
