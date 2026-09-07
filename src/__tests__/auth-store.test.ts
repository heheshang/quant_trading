import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { useAuthStore } from '@/stores/auth'
import { useOrderStore } from '@/stores/order'
import { useStrategyStore } from '@/stores/strategy'
import { useAccountStore } from '@/stores/account'
import { useRiskStore } from '@/stores/risk'
import { useStrategyLifecycleStore } from '@/stores/strategyLifecycle'
import type { Order, StrategyParams } from '@/services/types'

const { mockVerifyToken, mockLogin, mockProfile, mockStop, mockClear } = vi.hoisted(() => ({
  mockVerifyToken: vi.fn(), mockLogin: vi.fn(), mockProfile: vi.fn(),
  mockStop: vi.fn(), mockClear: vi.fn(),
}))
vi.mock('@/services/auth', () => ({
  login: mockLogin, verifyToken: mockVerifyToken, getUserProfile: mockProfile,
}))
vi.mock('@/stores/marketData', () => ({
  useMarketDataStore: () => ({ stop: mockStop, clear: mockClear }),
}))

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej })
  return { promise, resolve, reject }
}

const mockInvoke = vi.mocked(invoke)

function orderFixture(orderId: number): Order {
  return {
    order_id: orderId, strategy_id: 'strategy', symbol: 'BTCUSDT', order_type: 'Market',
    side: 'Buy', price: null, quantity: 1, filled_quantity: 0, status: 'Pending',
    created_at: '2026-09-07T00:00:00Z', updated_at: '2026-09-07T00:00:00Z', commission: 0, slippage: 0,
  }
}

describe('auth store session boundaries', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    localStorage.clear()
    setActivePinia(createPinia())
    mockLogin.mockResolvedValue('verified-token')
    mockVerifyToken.mockResolvedValue(true)
    mockProfile.mockResolvedValue({ user_id: 41, username: 'alice' })
    mockStop.mockResolvedValue(undefined)
  })

  it('restores only after verification and a server identity, ignoring the persisted username', async () => {
    localStorage.setItem('authToken', 'saved-token')
    localStorage.setItem('username', 'mallory')
    const verification = deferred<boolean>()
    const profile = deferred<Record<string, unknown>>()
    mockVerifyToken.mockReturnValue(verification.promise)
    mockProfile.mockReturnValue(profile.promise)
    const auth = useAuthStore()
    const restoring = auth.restoreSession()
    await vi.waitFor(() => expect(mockVerifyToken).toHaveBeenCalled())
    expect(auth.currentUser).toBeNull()
    expect(auth.isAuthenticated).toBe(false)
    verification.resolve(true)
    await vi.waitFor(() => expect(mockProfile).toHaveBeenCalled())
    expect(auth.currentUser).toBeNull()
    expect(auth.token).toBeNull()
    profile.resolve({ user_id: 41, username: 'alice' })
    expect(await restoring).toBe(true)
    expect(auth.currentUser).toEqual({ id: 41, username: 'alice' })
    expect(localStorage.getItem('username')).toBe('alice')
  })

  it('does not publish a login until the verified server profile arrives', async () => {
    const profile = deferred<Record<string, unknown>>()
    mockProfile.mockReturnValue(profile.promise)
    const auth = useAuthStore()
    const login = auth.login('alias', 'password', true, '123456')
    await vi.waitFor(() => expect(mockProfile).toHaveBeenCalled())
    expect(auth.isLoggedIn).toBe(false)
    expect(localStorage.getItem('authToken')).toBeNull()
    profile.resolve({ user_id: 41, username: 'alice' })
    expect(await login).toBe('/dashboard')
    expect(auth.currentUser).toEqual({ id: 41, username: 'alice' })
  })

  it('preserves active caches during same-session revalidation and clears them on rejection', async () => {
    const auth = useAuthStore()
    await auth.login('alice', 'password', false)
    const orders = useOrderStore()
    orders.activeOrders = [orderFixture(41)]
    const identity = auth.currentUser
    const verification = deferred<boolean>()
    mockVerifyToken.mockReturnValueOnce(verification.promise)
    const restoring = auth.restoreSession()
    expect(orders.activeOrders.map(order => order.order_id)).toEqual([41])
    expect(auth.currentUser).toBe(identity)
    verification.resolve(true)
    expect(await restoring).toBe(true)
    expect(orders.activeOrders.map(order => order.order_id)).toEqual([41])
    expect(auth.currentUser).toBe(identity)
    mockVerifyToken.mockResolvedValue(false)
    expect(await auth.restoreSession()).toBe(false)
    expect(orders.activeOrders).toEqual([])
    expect(auth.currentUser).toBeNull()
  })

  it.each(['invalid-token', 'verification-error', 'profile-error', 'unknown-id'])(
    'fails closed restoring %s', async (failure) => {
      localStorage.setItem('authToken', 'saved-token')
      if (failure === 'invalid-token') mockVerifyToken.mockResolvedValue(false)
      if (failure === 'verification-error') mockVerifyToken.mockRejectedValue(new Error('offline'))
      if (failure === 'profile-error') mockProfile.mockRejectedValue(new Error('profile unavailable'))
      if (failure === 'unknown-id') mockProfile.mockResolvedValue({ user_id: 0, username: 'alice' })
      const auth = useAuthStore()
      expect(await auth.restoreSession()).toBe(false)
      expect(auth.currentUser).toBeNull()
      expect(auth.isAuthenticated).toBe(false)
      expect(localStorage.getItem('authToken')).toBeNull()
    },
  )

  it('rejects login with an unknown profile rather than publishing id zero', async () => {
    mockProfile.mockResolvedValue({ username: 'alice' })
    const auth = useAuthStore()
    await expect(auth.login('alice', 'password', false)).rejects.toThrow()
    expect(auth.currentUser).toBeNull()
    expect(auth.isLoggedIn).toBe(false)
    expect(auth.loading).toBe(false)
    expect(localStorage.getItem('authToken')).toBeNull()
  })

  it('cannot resurrect a session when its profile arrives after logout', async () => {
    const profile = deferred<Record<string, unknown>>()
    mockProfile.mockReturnValue(profile.promise)
    const auth = useAuthStore()
    const login = auth.login('alice', 'password', false)
    const rejectedLogin = expect(login).rejects.toThrow()
    await vi.waitFor(() => expect(mockProfile).toHaveBeenCalled())
    await auth.clearSession()
    profile.resolve({ user_id: 41, username: 'alice' })
    await rejectedLogin
    expect(auth.currentUser).toBeNull()
    expect(localStorage.getItem('authToken')).toBeNull()
  })

  it('clears sensitive caches synchronously and refetches the next account without force', async () => {
    const auth = useAuthStore()
    const orders = useOrderStore()
    const strategies = useStrategyStore()
    const account = useAccountStore()
    const risk = useRiskStore()
    let owner = 'alice'
    mockInvoke.mockImplementation(async (command) => {
      if (command === 'get_active_orders') return [orderFixture(owner === 'alice' ? 41 : 52)]
      if (command === 'get_strategies') return [{ strategy_id: owner }]
      if (command === 'get_account_info') return { total_assets: owner === 'alice' ? 100 : 200 }
      if (command === 'get_risk_metrics') return { var_95: owner === 'alice' ? 1 : 2 }
      return []
    })
    await auth.login('alice', 'password', false)
    await Promise.all([orders.fetchActiveOrders(), strategies.fetchStrategies(), account.fetchAccountInfo(), risk.fetchMetrics()])
    const stopping = deferred<void>()
    mockStop.mockReturnValueOnce(stopping.promise)
    const logout = auth.clearSession()
    expect(orders.activeOrders).toEqual([])
    expect(strategies.strategies).toEqual([])
    expect(account.accountInfo).toBeNull()
    expect(risk.metrics).toEqual({})
    stopping.resolve()
    await logout
    owner = 'bob'
    mockProfile.mockResolvedValue({ user_id: 52, username: 'bob' })
    await auth.login('bob', 'password', false)
    await Promise.all([orders.fetchActiveOrders(), strategies.fetchStrategies(), account.fetchAccountInfo(), risk.fetchMetrics()])
    expect(orders.activeOrders[0].order_id).toBe(52)
    expect(strategies.strategies[0].strategy_id).toBe('bob')
    expect(account.totalAssets).toBe(200)
    expect(risk.var95).toBe(2)
  })

  it('ignores old requests and keeps the new session loading until its own response', async () => {
    const oldOrders = deferred<Order[]>()
    const oldStrategies = deferred<StrategyParams[]>()
    const newOrders = deferred<Order[]>()
    const newStrategies = deferred<StrategyParams[]>()
    mockInvoke.mockImplementation((command) => {
      if (command === 'get_active_orders') return oldOrders.promise
      if (command === 'get_strategies') return oldStrategies.promise
      return Promise.resolve([])
    })
    const auth = useAuthStore()
    const orders = useOrderStore()
    const strategies = useStrategyStore()
    const oldRequests = Promise.all([orders.fetchActiveOrders(), strategies.fetchStrategies()])
    await auth.clearSession()
    mockInvoke.mockImplementation((command) => command === 'get_active_orders' ? newOrders.promise : newStrategies.promise)
    const newRequests = Promise.all([orders.fetchActiveOrders(), strategies.fetchStrategies()])
    oldOrders.resolve([orderFixture(41)])
    oldStrategies.resolve([{ strategy_id: 'alice' } as StrategyParams])
    await oldRequests
    expect(orders.activeOrders).toEqual([])
    expect(strategies.strategies).toEqual([])
    expect(orders.loading).toBe(true)
    expect(strategies.loading.list).toBe(true)
    newOrders.resolve([orderFixture(52)])
    newStrategies.resolve([{ strategy_id: 'bob' } as StrategyParams])
    await newRequests
    expect(orders.activeOrders[0].order_id).toBe(52)
    expect(strategies.strategies[0].strategy_id).toBe('bob')
  })

  it('does not let a late lifecycle toggle mutate the replacement account collection', async () => {
    const toggled = deferred<boolean>()
    mockInvoke.mockReturnValue(toggled.promise)
    const strategies = useStrategyStore()
    const lifecycle = useStrategyLifecycleStore()
    const pending = lifecycle.toggleStrategy('same-id', true)
    await useAuthStore().clearSession()
    strategies.strategies = [{ strategy_id: 'same-id', enabled: false } as StrategyParams]
    toggled.resolve(true)
    await pending
    expect(strategies.strategies[0].enabled).toBe(false)
  })

  it('clears local session and caches even when stopping the backend stream fails', async () => {
    const auth = useAuthStore()
    await auth.login('alice', 'password', false)
    useOrderStore().activeOrders = [orderFixture(41)]
    mockClear.mockClear()
    mockStop.mockRejectedValue(new Error('stop failed'))
    await expect(auth.clearSession()).rejects.toThrow('stop failed')
    expect(auth.currentUser).toBeNull()
    expect(useOrderStore().activeOrders).toEqual([])
    expect(mockClear).toHaveBeenCalledOnce()
    await vi.waitFor(() => expect(localStorage.getItem('authToken')).toBeNull())
  })
})
