import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getAccountInfo, getPositions } from '@/services/account'
import type { AccountInfo, Position } from '@/services/types'

const CACHE_TTL_MS = 30_000

export const useAccountStore = defineStore('account', () => {
  const accountInfo = ref<AccountInfo | null>(null)
  const positions = ref<Position[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const lastFetched = ref(0)
  let generation = 0

  const isStale = computed(() => Date.now() - lastFetched.value > CACHE_TTL_MS)

  const totalAssets = computed(() => accountInfo.value?.total_assets ?? 0)
  const dailyPnl = computed(() => accountInfo.value?.daily_pnl ?? 0)
  const availableCash = computed(() => accountInfo.value?.available_cash ?? 0)
  const marketValue = computed(() => accountInfo.value?.market_value ?? 0)
  const totalPnl = computed(() => accountInfo.value?.total_pnl ?? 0)

  async function fetchAccountInfo(force = false) {
    if (!force && accountInfo.value && !isStale.value) return
    const requestGeneration = generation
    loading.value = true
    error.value = null
    try {
      const account = await getAccountInfo()
      if (requestGeneration !== generation) return
      accountInfo.value = account
      lastFetched.value = Date.now()
    } catch (err) {
      if (requestGeneration !== generation) return
      error.value = '获取账户信息失败'
      console.error('Failed to fetch account info:', err)
    } finally {
      if (requestGeneration === generation) loading.value = false
    }
  }

  async function fetchPositions(force = false) {
    if (!force && positions.value.length > 0 && !isStale.value) return
    const requestGeneration = generation
    try {
      const result = await getPositions()
      if (requestGeneration === generation) positions.value = result
    } catch (err) {
      if (requestGeneration !== generation) return
      console.error('Failed to fetch positions:', err)
    }
  }

  async function refreshAll() {
    const requestGeneration = generation
    error.value = null
    loading.value = true
    try {
      const [account, pos] = await Promise.all([getAccountInfo(), getPositions()])
      if (requestGeneration !== generation) return
      accountInfo.value = account
      positions.value = pos
      lastFetched.value = Date.now()
    } catch (err) {
      if (requestGeneration !== generation) return
      error.value = '刷新数据失败'
      console.error('Error refreshing account data:', err)
    } finally {
      if (requestGeneration === generation) loading.value = false
    }
  }

  function reset() {
    generation += 1
    accountInfo.value = null
    positions.value = []
    loading.value = false
    error.value = null
    lastFetched.value = 0
  }

  return {
    accountInfo,
    positions,
    loading,
    error,
    isStale,
    totalAssets,
    dailyPnl,
    availableCash,
    marketValue,
    totalPnl,
    fetchAccountInfo,
    fetchPositions,
    refreshAll,
    reset,
  }
})
