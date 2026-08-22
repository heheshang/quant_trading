import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const mockVerifyToken = vi.fn()

vi.mock('@/services/auth', () => ({
  login: vi.fn(),
  verifyToken: mockVerifyToken,
}))

describe('auth store session restore', () => {
  beforeEach(() => {
    vi.resetModules()
    localStorage.clear()
    setActivePinia(createPinia())
    mockVerifyToken.mockReset()
  })

  it('clears the persisted session when the token is invalid', async () => {
    localStorage.setItem('authToken', 'stale-token')
    localStorage.setItem('isAuthenticated', 'true')
    localStorage.setItem('username', 'admin')
    mockVerifyToken.mockResolvedValue(false)

    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()

    const isValid = await authStore.restoreSession()

    expect(isValid).toBe(false)
    expect(authStore.isAuthenticated).toBe(false)
    expect(authStore.token).toBeNull()
    expect(localStorage.getItem('authToken')).toBeNull()
  })

  it('keeps the persisted session when token verification fails transiently', async () => {
    localStorage.setItem('authToken', 'persisted-token')
    localStorage.setItem('isAuthenticated', 'true')
    localStorage.setItem('username', 'admin')
    mockVerifyToken.mockRejectedValue(new Error('backend unavailable'))

    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()

    const isValid = await authStore.restoreSession()

    // fail-open：瞬时错误不应清除已持久化会话
    expect(isValid).toBe(true)
    expect(authStore.isAuthenticated).toBe(true)
    expect(authStore.token).toBe('persisted-token')
    expect(localStorage.getItem('authToken')).toBe('persisted-token')
  })
})
