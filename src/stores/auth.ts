import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { login as apiLogin, verifyToken as apiVerifyToken } from '@/services/auth'
import { useMarketDataStore } from '@/stores/marketData'
import { getItem as secGet, setItem as secSet, removeItem as secRemove } from '@/services/secureStorage'

/**
 * Authentication store.
 *
 * 会话经 `plugin-store`（WebView 不可直接读的文件存储）持久化；非 Tauri/测试
 * 回退 localStorage。get/set/remove 均为异步。
 */

const STORAGE_KEYS = {
  AUTH_TOKEN: 'authToken',
  USERNAME: 'username',
  IS_AUTHENTICATED: 'isAuthenticated',
  REMEMBERED_USERNAME: 'remembered_username',
  REDIRECT_AFTER_LOGIN: 'redirect_after_login',
} as const

export const useAuthStore = defineStore('auth', () => {
  // ── State（初始为空，restoreSession 异步填充）──
  const token = ref<string | null>(null)
  const username = ref<string>('管理员')
  const isAuthenticated = ref<boolean>(false)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // ── Getters ──
  const isLoggedIn = computed(() => isAuthenticated.value && !!token.value)

  /**
   * Current logged-in user, derived from the persisted session.
   *
   * `id` falls back to `0` when the JWT/session does not carry a numeric
   * subject — the backend (or a real JWT decoder wired in later) will provide
   * the authoritative id. Callers that need a real id should check for `null`
   * and treat `0` as "unknown".
   */
  const currentUser = computed<{ id: number; username: string } | null>(() => {
    if (!isLoggedIn.value) return null
    return { id: 0, username: username.value }
  })

  // ── Actions ──

  /** Check persisted auth on app start. */
  async function restoreSession(): Promise<boolean> {
    const persistedToken = await secGet(STORAGE_KEYS.AUTH_TOKEN)
    const persistedUsername = (await secGet(STORAGE_KEYS.USERNAME)) || '管理员'

    if (!persistedToken) {
      await clearSession()
      return false
    }

    token.value = persistedToken
    username.value = persistedUsername
    isAuthenticated.value = true

    try {
      const valid = await apiVerifyToken(persistedToken)
      if (!valid) {
        await clearSession()
        return false
      }
      return true
    } catch {
      // 安全：verify_token 无法确认有效 → 清除会话（fail-closed），
      // 避免保留可能已被撤销/过期的 token；用户可重新登录。
      await clearSession()
      return false
    }
  }

  /** Persist auth state after successful login. */
  async function persistSession(newToken: string, newUsername: string): Promise<void> {
    token.value = newToken
    username.value = newUsername
    isAuthenticated.value = true

    await secSet(STORAGE_KEYS.AUTH_TOKEN, newToken)
    await secSet(STORAGE_KEYS.USERNAME, newUsername)
    await secSet(STORAGE_KEYS.IS_AUTHENTICATED, 'true')
  }

  /**
   * Attempt login with username + password.
   * On success, persists session and returns the redirect path.
   */
  async function login(
    usernameInput: string,
    password: string,
    remember: boolean,
    code?: string,
  ): Promise<string> {
    loading.value = true
    error.value = null

    try {
      // 1. Call Tauri login command
      const newToken = await apiLogin(usernameInput, password, code)

      // 2. Verify token validity
      try {
        const valid = await apiVerifyToken(newToken)
        if (!valid) throw new Error('Token 验证失败')
      } catch (verifyErr) {
        // Token invalid — clean up and abort
        await clearSession()
        error.value = '登录验证失败，请重试'
        throw verifyErr
      }

      // 3. Persist session
      await persistSession(newToken, usernameInput)

      // 4. Handle "remember me"
      if (remember) {
        await secSet(STORAGE_KEYS.REMEMBERED_USERNAME, usernameInput)
      } else {
        await secRemove(STORAGE_KEYS.REMEMBERED_USERNAME)
      }

      // 5. Determine redirect
      const redirect = (await secGet(STORAGE_KEYS.REDIRECT_AFTER_LOGIN)) || '/dashboard'
      await secRemove(STORAGE_KEYS.REDIRECT_AFTER_LOGIN)

      return redirect
    } catch (err) {
      error.value = (err as Error).message || '登录失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  /** Clear all persisted auth data (logout). */
  async function clearSession(): Promise<void> {
    token.value = null
    username.value = '管理员'
    isAuthenticated.value = false

    await secRemove(STORAGE_KEYS.AUTH_TOKEN)
    await secRemove(STORAGE_KEYS.USERNAME)
    await secRemove(STORAGE_KEYS.IS_AUTHENTICATED)

    // 清空实时行情数据，避免重新登录后残留上一位用户的标的数据。
    try {
      useMarketDataStore().clear()
    } catch {
      // 非 Pinia 上下文（如测试/未初始化）忽略。
    }
  }

  /** Store the path user wanted before being redirected to login. */
  async function setRedirectPath(path: string): Promise<void> {
    await secSet(STORAGE_KEYS.REDIRECT_AFTER_LOGIN, path)
  }

  /** Get remembered username for pre-fill. */
  async function getRememberedUsername(): Promise<string | null> {
    return secGet(STORAGE_KEYS.REMEMBERED_USERNAME)
  }

  return {
    // State
    token,
    username,
    isAuthenticated,
    loading,
    error,
    // Getters
    isLoggedIn,
    currentUser,
    // Actions
    restoreSession,
    login,
    clearSession,
    setRedirectPath,
    getRememberedUsername,
  }
})
