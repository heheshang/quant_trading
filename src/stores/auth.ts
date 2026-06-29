import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { login as apiLogin, verifyToken as apiVerifyToken } from '@/services/api'

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

  // ── Actions ──

  /** Check persisted auth on app start. */
  function restoreSession(): void {
    token.value = getItem(STORAGE_KEYS.AUTH_TOKEN)
    username.value = getItem(STORAGE_KEYS.USERNAME) || '管理员'
    isAuthenticated.value = getItem(STORAGE_KEYS.IS_AUTHENTICATED) === 'true'
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
    // Actions
    restoreSession,
    login,
    clearSession,
    setRedirectPath,
    getRememberedUsername,
  }
})
