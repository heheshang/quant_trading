import { describe, it, expect } from 'vitest'
import { ref, computed } from 'vue'
import type { Ref, ComputedRef } from 'vue'
import { useStrategyStatus } from '@/composables/useStrategyStatus'
import type { StrategyStatus } from '@/services/types'

describe('useStrategyStatus composable', () => {
  it('returns Running → 运行中 (success) with VideoPlay icon', () => {
    const status = ref<StrategyStatus>('Running')
    const display = useStrategyStatus(status)
    expect(display.value.label).toBe('运行中')
    expect(display.value.type).toBe('success')
    // icon is a Vue component (object) for the VideoPlay icon
    expect(display.value.icon).toBeDefined()
  })

  it('returns Draft → 草稿 (info) with EditPen icon', () => {
    const status = ref<StrategyStatus>('Draft')
    const display = useStrategyStatus(status)
    expect(display.value.label).toBe('草稿')
    expect(display.value.type).toBe('info')
    expect(display.value.icon).toBeDefined()
  })

  it('returns Backtesting → 回测中 (warning) with DataAnalysis icon', () => {
    const status = ref<StrategyStatus>('Backtesting')
    const display = useStrategyStatus(status)
    expect(display.value.label).toBe('回测中')
    expect(display.value.type).toBe('warning')
    expect(display.value.icon).toBeDefined()
  })

  it('returns Deployed → 已部署 (primary) with Upload icon', () => {
    const status = ref<StrategyStatus>('Deployed')
    const display = useStrategyStatus(status)
    expect(display.value.label).toBe('已部署')
    expect(display.value.type).toBe('primary')
    expect(display.value.icon).toBeDefined()
  })

  it('returns Paused → 已暂停 (warning) with VideoPause icon', () => {
    const status = ref<StrategyStatus>('Paused')
    const display = useStrategyStatus(status)
    expect(display.value.label).toBe('已暂停')
    expect(display.value.type).toBe('warning')
    expect(display.value.icon).toBeDefined()
  })

  it('returns Archived → 已归档 (info) with Box icon', () => {
    const status = ref<StrategyStatus>('Archived')
    const display = useStrategyStatus(status)
    expect(display.value.label).toBe('已归档')
    expect(display.value.type).toBe('info')
    expect(display.value.icon).toBeDefined()
  })

  it('accepts ComputedRef and stays reactive', () => {
    const status = computed<StrategyStatus>(() => 'Running')
    const display: ComputedRef<{ label: string; type: string; icon: unknown }> = useStrategyStatus(status)
    expect(display.value.label).toBe('运行中')
  })

  it('accepts plain Ref and stays reactive', () => {
    const status: Ref<StrategyStatus> = ref('Draft')
    const display = useStrategyStatus(status)
    expect(display.value.label).toBe('草稿')
    // mutating ref should propagate
    status.value = 'Running'
    expect(display.value.label).toBe('运行中')
  })

  it('covers all 6 StrategyStatus enum values with display info', () => {
    const all: StrategyStatus[] = ['Draft', 'Backtesting', 'Deployed', 'Running', 'Paused', 'Archived']
    for (const s of all) {
      const display = useStrategyStatus(ref(s))
      expect(display.value.label).toBeTruthy()
      expect(['info', 'warning', 'primary', 'success', 'danger']).toContain(display.value.type)
      expect(display.value.icon).toBeDefined()
    }
  })
})
