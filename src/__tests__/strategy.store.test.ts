import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useStrategyStore } from '@/stores/strategy'
import { invoke } from '@tauri-apps/api/core'
import type { StrategyParams, StrategyTypeInfo } from '@/services/types'

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
  createMockStrategy({ strategy_id: 's3', strategy_name: 'Paused Strategy', strategy_type: 'Arbitrage', enabled: true, status: 'Paused' }),
  createMockStrategy({ strategy_id: 's4', strategy_name: 'No Status Strategy', strategy_type: 'Custom', enabled: false }), // status undefined
  createMockStrategy({ strategy_id: 's5', strategy_name: 'Archived Strategy', strategy_type: 'MarketMaking', enabled: false, status: 'Archived' }),
  createMockStrategy({ strategy_id: 's6', strategy_name: 'Deployed Strategy', strategy_type: 'Statistical', enabled: true, status: 'Deployed' }),
  createMockStrategy({ strategy_id: 's7', strategy_name: 'Backtesting Strategy', strategy_type: 'MachineLearning', enabled: true, status: 'Backtesting' }),
]

const mockStrategyTypes: StrategyTypeInfo[] = [
  {
    type_name: 'TrendFollowing',
    display_name: '趋势跟随',
    description: 'Trend following strategy using moving averages',
    parameters: [
      { name: 'lookback', param_type: 'Number', default: 20, range: { min: 1, max: 100, step: 1 }, description: 'Lookback period' },
      { name: 'method', param_type: { Select: ['sma', 'ema'] }, default: 'sma', description: 'Method' },
    ],
  },
  {
    type_name: 'MeanReversion',
    display_name: '均值回归',
    description: 'Mean reversion strategy using Bollinger Bands',
    parameters: [
      { name: 'threshold', param_type: 'Number', default: 2.0, range: { min: 0.5, max: 5.0, step: 0.1 }, description: 'Entry threshold' },
    ],
  },
]

const mockStrategyTypeInfo: StrategyTypeInfo = mockStrategyTypes[0]

function setupDefaultMock() {
  mockInvoke.mockImplementation(async (cmd: string, _args?: unknown) => {
    switch (cmd) {
      case 'get_strategies': return [...mockStrategies]
      case 'save_strategy': return 'new-id'
      case 'delete_strategy': return true
      case 'start_strategy': return 'started'
      case 'stop_strategy': return 'stopped'
      case 'pause_strategy': return 'paused'
      case 'resume_strategy': return 'resumed'
      case 'deploy_strategy': return 'deployed'
      case 'archive_strategy': return 'archived'
      case 'toggle_strategy': return true
      case 'create_strategy': return 'new-id'
      case 'list_strategy_types': return [...mockStrategyTypes]
      case 'get_strategy_type_info': return { ...mockStrategyTypeInfo }
      default: return {}
    }
  })
}

describe('strategyStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    setupDefaultMock()
  })

  // ── fetchStrategies ──

  it('fetchStrategies fetches when strategies are empty and force=false', async () => {
    const store = useStrategyStore()
    expect(store.strategies).toHaveLength(0)
    await store.fetchStrategies(false)
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
    expect(store.strategies).toHaveLength(7)
  })

  it('fetchStrategies does NOT fetch when cached and force=false', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(false)
    expect(store.strategies).toHaveLength(7)
    mockInvoke.mockClear()
    await store.fetchStrategies(false)
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('fetchStrategies always fetches when force=true even with cache', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(false)
    expect(store.strategies).toHaveLength(7)
    mockInvoke.mockClear()
    await store.fetchStrategies(true)
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('fetchStrategies returns empty array when API returns no data', async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_strategies') return []
      return {}
    })
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    expect(store.strategies).toEqual([])
  })

  // ── loading state ──

  it('sets loading=true during fetch and false after', async () => {
    const store = useStrategyStore()
    // Use an unresolved promise to test loading state
    mockInvoke.mockImplementation(() => new Promise(() => {})) // never resolves
    store.fetchStrategies(true)
    expect(store.loading).toBe(true)
  })

  it('sets loading=false after fetch completes', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    expect(store.loading).toBe(false)
  })

  // ── error state ──

  it('sets error message on fetch failure', async () => {
    mockInvoke.mockRejectedValue(new Error('Network error'))
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    expect(store.error).toBe('获取策略列表失败')
  })

  it('sets error message on createStrategy failure', async () => {
    mockInvoke.mockRejectedValue(new Error('Save failed'))
    const store = useStrategyStore()
    await expect(store.createStrategy(createMockStrategy({ strategy_id: 'new' }))).rejects.toThrow()
    expect(store.error).toBe('创建策略失败')
  })

  it('sets error message on updateStrategy failure', async () => {
    mockInvoke.mockRejectedValue(new Error('Update failed'))
    const store = useStrategyStore()
    await expect(store.updateStrategy(createMockStrategy({ strategy_id: 's1' }))).rejects.toThrow()
    expect(store.error).toBe('更新策略失败')
  })

  it('sets error message on deleteStrategy failure', async () => {
    mockInvoke.mockRejectedValue(new Error('Delete failed'))
    const store = useStrategyStore()
    await expect(store.deleteStrategy('s1')).rejects.toThrow()
    expect(store.error).toBe('删除策略失败')
  })

  // ── createStrategy ──

  it('createStrategy calls saveStrategy and refreshes', async () => {
    const store = useStrategyStore()
    const newStrategy = createMockStrategy({ strategy_id: 'new' })
    await store.createStrategy(newStrategy)
    expect(mockInvoke).toHaveBeenCalledWith('save_strategy', { strategy: newStrategy })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
    expect(store.strategies).toHaveLength(7)
  })

  // ── updateStrategy ──

  it('updateStrategy calls saveStrategy and refreshes', async () => {
    const store = useStrategyStore()
    const updated = createMockStrategy({ strategy_id: 's1', strategy_name: 'Updated' })
    await store.updateStrategy(updated)
    expect(mockInvoke).toHaveBeenCalledWith('save_strategy', { strategy: updated })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  // ── deleteStrategy ──

  it('deleteStrategy calls deleteStrategy API and refreshes', async () => {
    const store = useStrategyStore()
    await store.deleteStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('delete_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  // ── Lifecycle actions ──

  it('startStrategy calls API and refreshes', async () => {
    const store = useStrategyStore()
    await store.startStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('start_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('stopStrategy calls API and refreshes', async () => {
    const store = useStrategyStore()
    await store.stopStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('stop_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('pauseStrategy calls API and refreshes', async () => {
    const store = useStrategyStore()
    await store.pauseStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('pause_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('resumeStrategy calls API and refreshes', async () => {
    const store = useStrategyStore()
    await store.resumeStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('resume_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('deployStrategy calls API and refreshes', async () => {
    const store = useStrategyStore()
    await store.deployStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('deploy_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('archiveStrategy calls API and refreshes', async () => {
    const store = useStrategyStore()
    await store.archiveStrategy('s1')
    expect(mockInvoke).toHaveBeenCalledWith('archive_strategy', { strategyId: 's1' })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
  })

  it('lifecycle actions set error on failure', async () => {
    const store = useStrategyStore()
    mockInvoke.mockRejectedValue(new Error('Fail'))
    await expect(store.startStrategy('s1')).rejects.toThrow()
    expect(store.error).toBe('启动策略失败')
    await expect(store.stopStrategy('s1')).rejects.toThrow()
    expect(store.error).toBe('停止策略失败')
    await expect(store.pauseStrategy('s1')).rejects.toThrow()
    expect(store.error).toBe('暂停策略失败')
    await expect(store.resumeStrategy('s1')).rejects.toThrow()
    expect(store.error).toBe('恢复策略失败')
    await expect(store.deployStrategy('s1')).rejects.toThrow()
    expect(store.error).toBe('部署策略失败')
    await expect(store.archiveStrategy('s1')).rejects.toThrow()
    expect(store.error).toBe('归档策略失败')
  })

  // ── toggleStrategy ──

  it('toggleStrategy toggles enabled locally', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    const s1 = store.strategies.find((s) => s.strategy_id === 's1')!
    expect(s1.enabled).toBe(true)
    await store.toggleStrategy('s1', false)
    expect(s1.enabled).toBe(false)
    expect(mockInvoke).toHaveBeenCalledWith('toggle_strategy', { strategyId: 's1', enabled: false })
  })

  it('toggleStrategy toggles enabled to true', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    const s2 = store.strategies.find((s) => s.strategy_id === 's2')!
    expect(s2.enabled).toBe(false)
    await store.toggleStrategy('s2', true)
    expect(s2.enabled).toBe(true)
  })

  it('toggleStrategy sets error on failure', async () => {
    const store = useStrategyStore()
    mockInvoke.mockRejectedValue(new Error('Toggle failed'))
    await expect(store.toggleStrategy('s1', false)).rejects.toThrow()
    expect(store.error).toBe('更新策略状态失败')
  })

  // ── Computed properties ──

  it('runningStrategies only includes Running strategies', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    expect(store.runningStrategies).toHaveLength(1)
    expect(store.runningStrategies[0].strategy_id).toBe('s1')
  })

  it('draftStrategies includes Draft and undefined status', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    expect(store.draftStrategies).toHaveLength(2)
    const ids = store.draftStrategies.map((s) => s.strategy_id)
    expect(ids).toContain('s2') // Draft
    expect(ids).toContain('s4') // status undefined
  })

  it('strategyById finds strategy by id', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    const found = store.strategyById('s3')
    expect(found).toBeDefined()
    expect(found!.strategy_name).toBe('Paused Strategy')
  })

  it('strategyById returns undefined for non-existent id', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    expect(store.strategyById('nonexistent')).toBeUndefined()
  })

  // ── Polling ──

  it('startPolling sets up interval that calls fetchStrategies', async () => {
    vi.useFakeTimers()
    const store = useStrategyStore()
    store.startPolling()
    expect(mockInvoke).not.toHaveBeenCalled()
    vi.advanceTimersByTime(5000)
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
    vi.useRealTimers()
  })

  it('stopPolling clears the interval', async () => {
    vi.useFakeTimers()
    const store = useStrategyStore()
    store.startPolling()
    store.stopPolling()
    vi.advanceTimersByTime(5000)
    expect(mockInvoke).not.toHaveBeenCalled()
    vi.useRealTimers()
  })

  it('startPolling is idempotent (does not start multiple intervals)', async () => {
    vi.useFakeTimers()
    const store = useStrategyStore()
    store.startPolling()
    store.startPolling() // second call should be no-op
    vi.advanceTimersByTime(5000)
    expect(mockInvoke).toHaveBeenCalledTimes(1) // only one setInterval fired
    vi.useRealTimers()
  })

  // ── initial state ──

  it('has initial empty state', () => {
    const store = useStrategyStore()
    expect(store.strategies).toEqual([])
    expect(store.currentStrategy).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
    expect(store.runningStrategies).toEqual([])
    expect(store.draftStrategies).toEqual([])
  })

  // ── selectStrategy (by id) ──

  it('selectStrategy sets currentStrategy when found', async () => {
    const store = useStrategyStore()
    await store.fetchStrategies(true)
    expect(store.currentStrategy).toBeNull()
    store.selectStrategy('s1')
    // selectStrategy reads from strategies cache synchronously
    expect(store.currentStrategy).toBeDefined()
    expect(store.currentStrategy!.strategy_id).toBe('s1')
  })

  it('selectStrategy handles errors gracefully', async () => {
    const store = useStrategyStore()
    // Ensure strategies are empty so the find returns undefined
    store.selectStrategy('s1')
    // No error should be set since no API call is made (looks in cache only)
    // Actually selectStrategy does: const found = strategies.value.find(...)
    // This won't throw, so error stays null
    // But if there's an error thrown by find (unlikely), it would be caught
    expect(store.error).toBeNull()
  })

  // ── createNewStrategy ──

  it('createNewStrategy calls create_strategy API with all args and refreshes', async () => {
    const store = useStrategyStore()
    const id = await store.createNewStrategy(
      'TrendFollowing',
      'My Strategy',
      { lookback: 20, method: 'sma' },
      true,
      50000,
      2500,
      1,
      'instance-1',
      'A test strategy',
      ['tag1', 'tag2'],
      ['BTC/USDT', 'ETH/USDT'],
    )
    expect(mockInvoke).toHaveBeenCalledWith('create_strategy', {
      typeName: 'TrendFollowing',
      strategyName: 'My Strategy',
      params: { lookback: 20, method: 'sma' },
      enabled: true,
      maxPosition: 50000,
      maxDailyLoss: 2500,
      instanceLabel: 'instance-1',
      description: 'A test strategy',
      tags: ['tag1', 'tag2'],
      symbols: ['BTC/USDT', 'ETH/USDT'],
      userId: 1,
    })
    expect(mockInvoke).toHaveBeenCalledWith('get_strategies')
    expect(id).toBe('new-id')
  })

  it('createNewStrategy works without optional args', async () => {
    const store = useStrategyStore()
    const id = await store.createNewStrategy(
      'MeanReversion',
      'Simple Strategy',
      { threshold: 2.0 },
      false,
      100000,
      5000,
      1,
    )
    expect(mockInvoke).toHaveBeenCalledWith('create_strategy', {
      typeName: 'MeanReversion',
      strategyName: 'Simple Strategy',
      params: { threshold: 2.0 },
      enabled: false,
      maxPosition: 100000,
      maxDailyLoss: 5000,
      instanceLabel: null,
      description: null,
      tags: [],
      symbols: [],
      userId: 1,
    })
    expect(id).toBe('new-id')
  })

  it('createNewStrategy sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('Create failed'))
    const store = useStrategyStore()
    await expect(store.createNewStrategy('TrendFollowing', 'Fail', {}, true, 0, 0, 1)).rejects.toThrow()
    expect(store.error).toBe('创建策略失败')
  })

  // ── listStrategyTypes ──

  it('listStrategyTypes populates strategyTypes', async () => {
    const store = useStrategyStore()
    expect(store.strategyTypes).toHaveLength(0)
    await store.listStrategyTypes()
    expect(mockInvoke).toHaveBeenCalledWith('list_strategy_types')
    expect(store.strategyTypes).toHaveLength(2)
    expect(store.strategyTypes[0].type_name).toBe('TrendFollowing')
    expect(store.strategyTypes[1].type_name).toBe('MeanReversion')
    expect(store.loading).toBe(false)
  })

  it('listStrategyTypes sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('List failed'))
    const store = useStrategyStore()
    await store.listStrategyTypes()
    expect(store.error).toBe('获取策略类型列表失败')
    expect(store.strategyTypes).toHaveLength(0)
  })

  // ── fetchStrategyTypeInfo ──

  it('fetchStrategyTypeInfo populates currentTypeInfo', async () => {
    const store = useStrategyStore()
    expect(store.currentTypeInfo).toBeNull()
    await store.fetchStrategyTypeInfo('TrendFollowing')
    expect(mockInvoke).toHaveBeenCalledWith('get_strategy_type_info', { typeName: 'TrendFollowing' })
    expect(store.currentTypeInfo).not.toBeNull()
    expect(store.currentTypeInfo!.type_name).toBe('TrendFollowing')
    expect(store.currentTypeInfo!.display_name).toBe('趋势跟随')
    expect(store.currentTypeInfo!.parameters).toHaveLength(2)
    expect(store.loading).toBe(false)
  })

  it('fetchStrategyTypeInfo sets error on failure', async () => {
    mockInvoke.mockRejectedValue(new Error('Info failed'))
    const store = useStrategyStore()
    await store.fetchStrategyTypeInfo('UnknownType')
    expect(store.error).toBe('获取策略类型信息失败')
    expect(store.currentTypeInfo).toBeNull()
  })
})
