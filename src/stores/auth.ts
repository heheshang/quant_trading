import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { login as apiLogin, verifyToken as apiVerifyToken, getUserProfile } from '@/services/auth'
import { useMarketDataStore } from '@/stores/marketData'
import { useOrderStore } from '@/stores/order'
import { useStrategyStore } from '@/stores/strategy'
import { useAccountStore } from '@/stores/account'
import { useRiskStore } from '@/stores/risk'
import { useStrategyLifecycleStore } from '@/stores/strategyLifecycle'
import { getItem as secGet, setItem as secSet, removeItem as secRemove } from '@/services/secureStorage'

const STORAGE_KEYS = {
  AUTH_TOKEN: 'authToken',
  USERNAME: 'username',
  IS_AUTHENTICATED: 'isAuthenticated',
  REMEMBERED_USERNAME: 'remembered_username',
  REDIRECT_AFTER_LOGIN: 'redirect_after_login',
} as const

type SessionUser = { id: number; username: string }

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(null)
  const username = ref('管理员')
  const isAuthenticated = ref(false)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const verifiedUser = ref<SessionUser | null>(null)
  let generation = 0
  // Serialize writes/removals so an interrupted login cannot persist after logout.
  let sessionWrites: Promise<void> = Promise.resolve()

  const isLoggedIn = computed(() => isAuthenticated.value && !!token.value && !!verifiedUser.value)
  const currentUser = computed(() => isLoggedIn.value ? verifiedUser.value : null)

  function resetSessionState() {
    token.value = null
    username.value = '管理员'
    isAuthenticated.value = false
    verifiedUser.value = null
    loading.value = false
    error.value = null
    useOrderStore().reset()
    useStrategyStore().reset()
    useAccountStore().reset()
    useRiskStore().reset()
    useStrategyLifecycleStore().reset()
  }

  function enqueueSessionWrite(operation: () => Promise<void>): Promise<void> {
    sessionWrites = sessionWrites.then(operation, operation)
    return sessionWrites
  }

  async function verifyIdentity(newToken: string): Promise<SessionUser> {
    if (!await apiVerifyToken(newToken)) throw new Error('Token 验证失败')
    const profile = await getUserProfile()
    if (typeof profile.user_id !== 'number' || !Number.isSafeInteger(profile.user_id) || profile.user_id <= 0 ||
        typeof profile.username !== 'string' || !profile.username.trim()) {
      throw new Error('无法确认登录用户身份')
    }
    return { id: profile.user_id, username: profile.username }
  }

  async function persistSession(newToken: string, user: SessionUser, attempt: number): Promise<boolean> {
    await enqueueSessionWrite(async () => {
      if (attempt !== generation) return
      await secSet(STORAGE_KEYS.AUTH_TOKEN, newToken)
      await secSet(STORAGE_KEYS.USERNAME, user.username)
      await secSet(STORAGE_KEYS.IS_AUTHENTICATED, 'true')
    })
    if (attempt !== generation) return false
    token.value = newToken
    username.value = user.username
    verifiedUser.value = user
    isAuthenticated.value = true
    return true
  }

  async function restoreSession(): Promise<boolean> {
    const attempt = ++generation
    try {
      await sessionWrites
      const persistedToken = await secGet(STORAGE_KEYS.AUTH_TOKEN)
      if (attempt !== generation) return false
      if (!persistedToken) throw new Error('没有已保存的会话')
      const user = await verifyIdentity(persistedToken)
      if (attempt !== generation) return false
      if (token.value === persistedToken && verifiedUser.value?.id === user.id &&
          verifiedUser.value.username === user.username && isLoggedIn.value) return true
      resetSessionState()
      return await persistSession(persistedToken, user, attempt)
    } catch {
      if (attempt === generation) await clearSession()
      return false
    }
  }

  async function login(
    usernameInput: string,
    password: string,
    remember: boolean,
    code?: string,
  ): Promise<string> {
    const cleanup = clearSession()
    const attempt = generation
    loading.value = true
    try {
      await cleanup
      if (attempt !== generation) throw new Error('登录会话已取消')
      const newToken = await apiLogin(usernameInput, password, code)
      if (attempt !== generation) throw new Error('登录会话已取消')
      const user = await verifyIdentity(newToken)
      if (attempt !== generation || !await persistSession(newToken, user, attempt)) {
        throw new Error('登录会话已取消')
      }
      await enqueueSessionWrite(async () => {
        if (attempt !== generation) return
        if (remember) await secSet(STORAGE_KEYS.REMEMBERED_USERNAME, usernameInput)
        else await secRemove(STORAGE_KEYS.REMEMBERED_USERNAME)
      })
      const redirect = (await secGet(STORAGE_KEYS.REDIRECT_AFTER_LOGIN)) || '/dashboard'
      if (attempt !== generation) throw new Error('登录会话已取消')
      await secRemove(STORAGE_KEYS.REDIRECT_AFTER_LOGIN)
      return redirect
    } catch (err) {
      if (attempt === generation) {
        const cleanup = clearSession()
        const clearedGeneration = generation
        await cleanup
        if (generation === clearedGeneration) error.value = (err as Error).message || '登录失败'
      }
      throw err
    } finally {
      if (attempt === generation) loading.value = false
    }
  }

  async function clearSession(): Promise<void> {
    generation += 1
    resetSessionState()
    const marketData = useMarketDataStore()
    const stopped = marketData.stop()
    const removed = enqueueSessionWrite(async () => {
      await secRemove(STORAGE_KEYS.AUTH_TOKEN)
      await secRemove(STORAGE_KEYS.USERNAME)
      await secRemove(STORAGE_KEYS.IS_AUTHENTICATED)
    })
    const results = await Promise.allSettled([removed, stopped.finally(() => marketData.clear())])
    for (const result of results) {
      if (result.status === 'rejected') throw result.reason
    }
  }

  async function setRedirectPath(path: string): Promise<void> {
    await secSet(STORAGE_KEYS.REDIRECT_AFTER_LOGIN, path)
  }

  async function getRememberedUsername(): Promise<string | null> {
    return secGet(STORAGE_KEYS.REMEMBERED_USERNAME)
  }

  return {
    token,
    username,
    isAuthenticated,
    loading,
    error,
    isLoggedIn,
    currentUser,
    restoreSession,
    login,
    clearSession,
    setRedirectPath,
    getRememberedUsername,
  }
})
