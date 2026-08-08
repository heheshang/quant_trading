import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { login as apiLogin, verifyToken as apiVerifyToken } from '@/services/auth'

/**
 * Authentication store.
 *
 * Backed by localStorage for now. Designed to support @tauri-apps/plugin-store
 * or tauri-plugin-secure-store as a drop-in replacement — swap the storage
 * adapter without changing the store's public API.
 */

const STORAGE_KEYS = {
  AUTH_TOKEN: 'authToken',
  USERNAME: 'username',
  IS_AUTHENTICATED: 'isAuthenticated',
  REMEMBERED_USERNAME: 'remembered_username',
  REDIRECT_AFTER_LOGIN: 'redirect_after_login',
} as const

function getItem(key: string): string | null {
  return localStorage.getItem(key)
}

function setItem(key: string, value: string): void {
  localStorage.setItem(key, value)
}

function removeItem(key: string): void {
  localStorage.removeItem(key)
}

export const useAuthStore = defineStore('auth', () => {
  // ── State ──
  const token = ref<string | null>(getItem(STORAGE_KEYS.AUTH_TOKEN))
  const username = ref<string>(getItem(STORAGE_KEYS.USERNAME) || '管理员')
  const isAuthenticated = ref<boolean>(getItem(STORAGE_KEYS.IS_AUTHENTICATED) === 'true')
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
    const persistedToken = getItem(STORAGE_KEYS.AUTH_TOKEN)
    const persistedUsername = getItem(STORAGE_KEYS.USERNAME) || '管理员'

    if (!persistedToken) {
      clearSession()
      return false
    }

    token.value = persistedToken
    username.value = persistedUsername
    isAuthenticated.value = true

    try {
      const valid = await apiVerifyToken(persistedToken)
      if (!valid) {
        clearSession()
        return false
      }

      setItem(STORAGE_KEYS.AUTH_TOKEN, persistedToken)
      setItem(STORAGE_KEYS.USERNAME, persistedUsername)
      setItem(STORAGE_KEYS.IS_AUTHENTICATED, 'true')
      return true
    } catch (err) {
      clearSession()
      return false
    }
  }

  /** Persist auth state after successful login. */
  function persistSession(newToken: string, newUsername: string): void {
    token.value = newToken
    username.value = newUsername
    isAuthenticated.value = true

    setItem(STORAGE_KEYS.AUTH_TOKEN, newToken)
    setItem(STORAGE_KEYS.USERNAME, newUsername)
    setItem(STORAGE_KEYS.IS_AUTHENTICATED, 'true')
  }

  /**
   * Attempt login with username + password.
   * On success, persists session and returns the redirect path.
   */
  async function login(usernameInput: string, password: string, remember: boolean): Promise<string> {
    loading.value = true
    error.value = null

    try {
      // 1. Call Tauri login command
      const newToken = await apiLogin(usernameInput, password)

      // 2. Verify token validity
      try {
        const valid = await apiVerifyToken(newToken)
        if (!valid) throw new Error('Token 验证失败')
      } catch (verifyErr) {
        // Token invalid — clean up and abort
        clearSession()
        error.value = '登录验证失败，请重试'
        throw verifyErr
      }

      // 3. Persist session
      persistSession(newToken, usernameInput)

      // 4. Handle "remember me"
      if (remember) {
        setItem(STORAGE_KEYS.REMEMBERED_USERNAME, usernameInput)
      } else {
        removeItem(STORAGE_KEYS.REMEMBERED_USERNAME)
      }

      // 5. Determine redirect
      const redirect = getItem(STORAGE_KEYS.REDIRECT_AFTER_LOGIN) || '/dashboard'
      removeItem(STORAGE_KEYS.REDIRECT_AFTER_LOGIN)

      return redirect
    } catch (err) {
      error.value = (err as Error).message || '登录失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  /** Clear all persisted auth data (logout). */
  function clearSession(): void {
    token.value = null
    username.value = '管理员'
    isAuthenticated.value = false

    removeItem(STORAGE_KEYS.AUTH_TOKEN)
    removeItem(STORAGE_KEYS.USERNAME)
    removeItem(STORAGE_KEYS.IS_AUTHENTICATED)
  }

  /** Store the path user wanted before being redirected to login. */
  function setRedirectPath(path: string): void {
    setItem(STORAGE_KEYS.REDIRECT_AFTER_LOGIN, path)
  }

  /** Get remembered username for pre-fill. */
  function getRememberedUsername(): string | null {
    return getItem(STORAGE_KEYS.REMEMBERED_USERNAME)
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
