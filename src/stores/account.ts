import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getAccountInfo, getPositions } from '@/services/api'
import type { AccountInfo, Position } from '@/services/types'

const CACHE_TTL_MS = 30_000

export const useAccountStore = defineStore('account', () => {
  const accountInfo = ref<AccountInfo | null>(null)
  const positions = ref<Position[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  let lastFetched = 0

  const isStale = computed(() => Date.now() - lastFetched > CACHE_TTL_MS)

  const totalAssets = computed(() => accountInfo.value?.total_assets ?? 0)
  const dailyPnl = computed(() => accountInfo.value?.daily_pnl ?? 0)
  const availableCash = computed(() => accountInfo.value?.available_cash ?? 0)
  const marketValue = computed(() => accountInfo.value?.market_value ?? 0)
  const totalPnl = computed(() => accountInfo.value?.total_pnl ?? 0)

  async function fetchAccountInfo(force = false) {
    if (!force && accountInfo.value && !isStale.value) return
    loading.value = true
    error.value = null
    try {
      accountInfo.value = await getAccountInfo()
      lastFetched = Date.now()
    } catch (err) {
      error.value = '获取账户信息失败'
      console.error('Failed to fetch account info:', err)
    } finally {
      loading.value = false
    }
  }

  async function fetchPositions(force = false) {
    if (!force && positions.value.length > 0 && !isStale.value) return
    try {
      positions.value = await getPositions()
    } catch (err) {
      console.error('Failed to fetch positions:', err)
    }
  }

  async function refreshAll() {
    error.value = null
    loading.value = true
    try {
      const [account, pos] = await Promise.all([getAccountInfo(), getPositions()])
      accountInfo.value = account
      positions.value = pos
      lastFetched = Date.now()
    } catch (err) {
      error.value = '刷新数据失败'
      console.error('Error refreshing account data:', err)
    } finally {
      loading.value = false
    }
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
  }
})
