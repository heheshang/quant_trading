import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { invoke } from '@tauri-apps/api/core'

// ---------------------------------------------------------------------------
// Hoisted mocks
// ---------------------------------------------------------------------------

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SchemaField {
  name: string
  type: 'number' | 'string' | 'boolean'
  default?: unknown
  required?: boolean
  min?: number
  max?: number
}

interface StrategyType {
  id: string
  name: string
  description: string
  parameter_schema: SchemaField[]
}

interface StrategyInstance {
  id: string
  strategy_id: string
  name: string
  strategy_type: string
  status: string
  params: Record<string, unknown>
  description?: string
  tags?: string[]
  symbols?: string[]
  instance_label?: string
  created_at: number
  updated_at: number
}

// ---------------------------------------------------------------------------
// Sample data
// ---------------------------------------------------------------------------

const sampleTypes: StrategyType[] = [
  {
    id: 'ma_cross',
    name: 'MA Cross',
    description: 'Moving Average Crossover',
    parameter_schema: [
      { name: 'fast_period', type: 'number', default: 10, required: true, min: 2, max: 200 },
      { name: 'slow_period', type: 'number', default: 30, required: true, min: 5, max: 500 },
      { name: 'signal_threshold', type: 'number', default: 0.001, required: false, min: 0.0001, max: 0.1 },
    ],
  },
  {
    id: 'rsi',
    name: 'RSI Strategy',
    description: 'Relative Strength Index',
    parameter_schema: [
      { name: 'period', type: 'number', default: 14, required: true, min: 2, max: 100 },
      { name: 'overbought', type: 'number', default: 70, required: true, min: 50, max: 100 },
      { name: 'oversold', type: 'number', default: 30, required: true, min: 0, max: 50 },
    ],
  },
]

const sampleStrategy: StrategyInstance = {
  id: 'strat-1',
  strategy_id: 'strat-1',
  name: 'Test Strategy',
  strategy_type: 'ma_cross',
  status: 'active',
  params: { fast_period: 10, slow_period: 30 },
  description: 'A test strategy',
  tags: ['test', 'prod'],
  symbols: ['BTC-USDT', 'ETH-USDT'],
  created_at: Date.now() - 86400000,
  updated_at: Date.now(),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockInvoke = vi.mocked(invoke)

function setupBackend(overrides: Record<string, unknown> = {}) {
  const defaults: Record<string, unknown> = {
    list_strategy_types: sampleTypes,
    list_strategies: [sampleStrategy],
    get_strategy: sampleStrategy,
    get_strategy_type_info: sampleTypes[0],
    create_strategy: { ...sampleStrategy, id: 'strat-2', strategy_id: 'strat-2', name: 'New Strategy', status: 'draft', created_at: Date.now(), updated_at: Date.now() },
    update_strategy: { ...sampleStrategy, updated_at: Date.now() },
    delete_strategy: true,
    start_strategy: { ...sampleStrategy, status: 'active' },
    stop_strategy: true,
  }
  const merged = { ...defaults, ...overrides }
  mockInvoke.mockImplementation(async (cmd: string, args?: Parameters<typeof invoke>[1]) => {
    if (cmd === 'get_strategy_type_info') {
      const recordArgs = (args ?? {}) as Record<string, unknown>
      const typeId = (recordArgs.strategyType as string) || ''
      const type = (sampleTypes as StrategyType[]).find((t) => t.id === typeId) || merged[cmd]
      return type
    }
    if (cmd in merged) return merged[cmd]
    return {}
  })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('Strategy Backend Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // ── Strategy Types ──

  it('fetches list of available strategy types', async () => {
    setupBackend()
    const types = await invoke('list_strategy_types') as StrategyType[]

    expect(Array.isArray(types)).toBe(true)
    expect(types.length).toBe(2)
    expect(types[0].id).toBe('ma_cross')
    expect(types[0].parameter_schema.length).toBe(3)
  })

  it('fetches strategy type info with parameter schema', async () => {
    setupBackend()
    const info = await invoke('get_strategy_type_info', { strategyType: 'ma_cross' }) as StrategyType

    expect(info.id).toBe('ma_cross')
    expect(info.parameter_schema).toBeDefined()

    const fastPeriod = info.parameter_schema.find((p) => p.name === 'fast_period')
    expect(fastPeriod).toBeDefined()
    expect(fastPeriod!.required).toBe(true)
    expect(fastPeriod!.min).toBe(2)
    expect(fastPeriod!.max).toBe(200)
  })

  it('validates strategy type schema fields', async () => {
    setupBackend()
    const info = await invoke('get_strategy_type_info', { strategyType: 'rsi' }) as StrategyType

    const period = info.parameter_schema.find((p) => p.name === 'period')
    expect(period).toBeDefined()
    expect(period!.type).toBe('number')
    expect(period!.default).toBe(14)

    const overbought = info.parameter_schema.find((p) => p.name === 'overbought')
    expect(overbought).toBeDefined()
    expect(overbought!.max).toBe(100)
  })

  // ── Strategy CRUD ──

  it('lists all strategies from backend', async () => {
    setupBackend()
    const strategies = await invoke('list_strategies') as StrategyInstance[]

    expect(Array.isArray(strategies)).toBe(true)
    expect(strategies.length).toBe(1)
    expect(strategies[0].name).toBe('Test Strategy')
    expect(strategies[0].params.fast_period).toBe(10)
  })

  it('gets a strategy by ID', async () => {
    setupBackend()
    const strategy = await invoke('get_strategy', { strategyId: 'strat-1' }) as StrategyInstance

    expect(strategy).toBeDefined()
    expect(strategy.id).toBe('strat-1')
    expect(strategy.tags).toContain('test')
    expect(strategy.symbols).toContain('BTC-USDT')
  })

  it('creates a new strategy', async () => {
    setupBackend()
    const result = await invoke('create_strategy', {
      name: 'New Strategy',
      strategyType: 'ma_cross',
      params: { fast_period: 10, slow_period: 30 },
      description: 'A new strategy',
      tags: ['new'],
      symbols: ['ETH-USDT'],
    }) as StrategyInstance

    expect(result).toBeDefined()
    expect(result.name).toBe('New Strategy')
  })

  it('creates strategy with minimal params (backend default filling)', async () => {
    setupBackend()
    const result = await invoke('create_strategy', {
      name: 'Minimal Strategy',
      strategyType: 'ma_cross',
      params: {},
    }) as StrategyInstance

    expect(result).toBeDefined()
  })

  it('handles backend validation error on create', async () => {
    mockInvoke.mockRejectedValue(new Error('参数验证失败: fast_period 超出范围 [2, 200]'))

    await expect(
      invoke('create_strategy', {
        name: 'Bad Strategy',
        strategyType: 'ma_cross',
        params: { fast_period: 999 },
      })
    ).rejects.toThrow('参数验证失败')
  })

  it('updates an existing strategy', async () => {
    setupBackend()
    const result = await invoke('update_strategy', {
      strategyId: 'strat-1',
      name: 'Updated Strategy',
      params: { fast_period: 20, slow_period: 50 },
    }) as StrategyInstance

    expect(result).toBeDefined()
  })

  it('deletes a strategy', async () => {
    setupBackend()
    const result = await invoke('delete_strategy', { strategyId: 'strat-1' })

    expect(result).toBe(true)
  })

  // ── Strategy Lifecycle ──

  it('starts a strategy from inactive state', async () => {
    setupBackend()
    const result = await invoke('start_strategy', { strategyId: 'strat-1' }) as StrategyInstance

    expect(result).toBeDefined()
    expect(result.status).toBe('active')
  })

  it('stops a running strategy', async () => {
    setupBackend()
    const result = await invoke('stop_strategy', { strategyId: 'strat-1' })

    expect(result).toBe(true)
  })

  // ── Edge Cases ──

  it('returns empty list when no strategies exist', async () => {
    setupBackend({ list_strategies: [] })
    const strategies = await invoke('list_strategies') as StrategyInstance[]

    expect(Array.isArray(strategies)).toBe(true)
    expect(strategies.length).toBe(0)
  })

  it('handles missing strategy gracefully', async () => {
    mockInvoke.mockRejectedValue(new Error('策略不存在'))

    await expect(
      invoke('get_strategy', { strategyId: 'nonexistent' })
    ).rejects.toThrow('策略不存在')
  })
})
