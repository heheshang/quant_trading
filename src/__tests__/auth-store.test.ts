import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const mockVerifyToken = vi.fn()
const mockLogin = vi.fn()

vi.mock('@/services/auth', () => ({
  login: mockLogin,
  verifyToken: mockVerifyToken,
}))

describe('auth store session restore', () => {
  beforeEach(() => {
    vi.resetModules()
    localStorage.clear()
    setActivePinia(createPinia())
    mockVerifyToken.mockReset()
    mockLogin.mockReset()
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

  it('clears the session when token verification fails (fail-closed)', async () => {
    localStorage.setItem('authToken', 'persisted-token')
    localStorage.setItem('isAuthenticated', 'true')
    localStorage.setItem('username', 'admin')
    mockVerifyToken.mockRejectedValue(new Error('backend unavailable'))

    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()

    const isValid = await authStore.restoreSession()

    // fail-closed：无法确认 token 有效 → 清除会话（安全，防留失效 token）
    expect(isValid).toBe(false)
    expect(authStore.isAuthenticated).toBe(false)
    expect(authStore.token).toBeNull()
    expect(localStorage.getItem('authToken')).toBeNull()
  })

  it('passes a provided 2FA code through to the login API', async () => {
    mockLogin.mockResolvedValue('token-123')
    mockVerifyToken.mockResolvedValue(true)

    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()

    await authStore.login('admin', 'password123', true, '123456')

    expect(mockLogin).toHaveBeenCalledWith('admin', 'password123', '123456')
  })

  it('omits the 2FA code when it is not provided', async () => {
    mockLogin.mockResolvedValue('token-123')
    mockVerifyToken.mockResolvedValue(true)

    const { useAuthStore } = await import('@/stores/auth')
    const authStore = useAuthStore()

    await authStore.login('admin', 'password123', false)

    expect(mockLogin).toHaveBeenCalledWith('admin', 'password123', undefined)
  })
})
