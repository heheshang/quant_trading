import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useStrategyStore } from '@/stores/strategy'
import { useStrategyLifecycleStore } from '@/stores/strategyLifecycle'
import { invoke } from '@tauri-apps/api/core'
import type { StrategyParams } from '@/services/types'

const mockInvoke = vi.mocked(invoke)

function createMockStrategy(overrides: Partial<Record<string, unknown>> = {}): StrategyParams {
  return {
    strategy_id: '',
    strategy_name: '',
    strategy_type: 'TrendFollowing' as StrategyParams['strategy_type'],
    enabled: false,
    max_position: 100000,
    max_daily_loss: 5000,
    params: {} as Record<string, unknown>,
    status: 'Draft' as StrategyParams['status'],
    tags: [] as string[],
    symbols: [] as string[],
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

const mockStrategies = [
  createMockStrategy({ strategy_id: 's1', strategy_name: 'Running Strategy', strategy_type: 'TrendFollowing', enabled: true, status: 'Running' }),
  createMockStrategy({ strategy_id: 's2', strategy_name: 'Draft Strategy', strategy_type: 'MeanReversion', enabled: false, status: 'Draft' }),
]

function setupDefaultMock() {
  mockInvoke.mockImplementation(async (cmd: string, _args?: unknown) => {
    switch (cmd) {
      case 'get_strategies': return [...mockStrategies]
      case 'start_strategy': return 'started'
      case 'stop_strategy': return 'stopped'
      case 'pause_strategy': return 'paused'
      case 'resume_strategy': return 'resumed'
      case 'deploy_strategy': return 'deployed'
      case 'archive_strategy': return 'archived'
      case 'toggle_strategy': return true
      default: return {}
    }
  })
}

describe('strategyLifecycleStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    setupDefaultMock()
  })

  it('startStrategy calls API and refreshes', async () => {
    const lifecycle = useStrategyLifecycleStore()
    await lifecycle.startStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('start_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('stopStrategy calls API and refreshes', async () => {
    const lifecycle = useStrategyLifecycleStore()
    await lifecycle.stopStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('stop_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('pauseStrategy calls API and refreshes', async () => {
    const lifecycle = useStrategyLifecycleStore()
    await lifecycle.pauseStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('pause_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('resumeStrategy calls API and refreshes', async () => {
    const lifecycle = useStrategyLifecycleStore()
    await lifecycle.resumeStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('resume_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('deployStrategy calls API and refreshes', async () => {
    const lifecycle = useStrategyLifecycleStore()
    await lifecycle.deployStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('deploy_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('archiveStrategy calls API and refreshes', async () => {
    const lifecycle = useStrategyLifecycleStore()
    await lifecycle.archiveStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('archive_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('lifecycle actions set per-action error on failure and do not affect base loading', async () => {
    mockInvoke.mockRejectedValue(new Error('Fail'))
    const lifecycle = useStrategyLifecycleStore()
    const base = useStrategyStore()
    await expect(lifecycle.startStrategy('s1')).rejects.toThrow()
    expect(lifecycle.error.start).toBe('启动策略失败')
    expect(base.loading.list).toBe(false)
    await expect(lifecycle.stopStrategy('s1')).rejects.toThrow()
    expect(lifecycle.error.stop).toBe('停止策略失败')
    await expect(lifecycle.pauseStrategy('s1')).rejects.toThrow()
    expect(lifecycle.error.pause).toBe('暂停策略失败')
    await expect(lifecycle.resumeStrategy('s1')).rejects.toThrow()
    expect(lifecycle.error.resume).toBe('恢复策略失败')
    await expect(lifecycle.deployStrategy('s1')).rejects.toThrow()
    expect(lifecycle.error.deploy).toBe('部署策略失败')
    await expect(lifecycle.archiveStrategy('s1')).rejects.toThrow()
    expect(lifecycle.error.archive).toBe('归档策略失败')
    expect(lifecycle.error.toggle).toBeNull()
  })

  it('toggleStrategy toggles enabled locally and calls API', async () => {
    const base = useStrategyStore()
    await base.fetchStrategies(true)
    const lifecycle = useStrategyLifecycleStore()
    const s1 = base.strategies.find((s) => s.strategy_id === 's1')!
    expect(s1.enabled).toBe(true)
    await lifecycle.toggleStrategy('s1', false)
    expect(s1.enabled).toBe(false)
    expect(mockInvoke).toHaveBeenCalledWith('toggle_strategy', { strategyId: 's1', enabled: false })
  })

  it('toggleStrategy toggles enabled to true', async () => {
    const base = useStrategyStore()
    await base.fetchStrategies(true)
    const lifecycle = useStrategyLifecycleStore()
    const s2 = base.strategies.find((s) => s.strategy_id === 's2')!
    expect(s2.enabled).toBe(false)
    await lifecycle.toggleStrategy('s2', true)
    expect(s2.enabled).toBe(true)
  })

  it('toggleStrategy sets error.toggle on failure', async () => {
    const lifecycle = useStrategyLifecycleStore()
    mockInvoke.mockRejectedValue(new Error('Toggle failed'))
    await expect(lifecycle.toggleStrategy('s1', false)).rejects.toThrow()
    expect(lifecycle.error.toggle).toBe('更新策略状态失败')
  })
})
