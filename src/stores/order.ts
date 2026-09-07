import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getActiveOrders, getRecentOrders, submitOrder } from '@/services/order'
import type { Order } from '@/services/types'

const CACHE_TTL_MS = 30_000

export const useOrderStore = defineStore('order', () => {
  const activeOrders = ref<Order[]>([])
  const recentOrders = ref<Order[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const lastFetched = ref(0)
  let generation = 0

  const isStale = computed(() => Date.now() - lastFetched.value > CACHE_TTL_MS)

  const orderCount = computed(() => activeOrders.value.length)

  const pendingOrders = computed(() =>
    activeOrders.value.filter((o: Order) => o.status === 'Pending' || o.status === 'Submitted'),
  )

  const filledOrders = computed(() =>
    activeOrders.value.filter((o: Order) => o.status === 'Filled' || o.status === 'PartiallyFilled'),
  )

  async function fetchActiveOrders(force = false) {
    if (!force && activeOrders.value.length > 0 && !isStale.value) return
    const requestGeneration = generation
    loading.value = true
    error.value = null
    try {
      const orders = await getActiveOrders()
      if (requestGeneration !== generation) return
      activeOrders.value = orders
      lastFetched.value = Date.now()
    } catch (err) {
      if (requestGeneration !== generation) return
      error.value = '获取订单信息失败'
      console.error('Failed to fetch active orders:', err)
    } finally {
      if (requestGeneration === generation) loading.value = false
    }
  }

  async function fetchRecentOrders(force = false) {
    if (!force && recentOrders.value.length > 0 && !isStale.value) return
    const requestGeneration = generation
    try {
      const orders = await getRecentOrders(50)
      if (requestGeneration === generation) recentOrders.value = orders
    } catch {
      // 忽略：历史拉取失败不阻塞页面
    }
  }

  async function placeOrder(order: Order): Promise<string | null> {
    const requestGeneration = generation
    try {
      const orderId = await submitOrder(order)
      if (requestGeneration !== generation) return null
      await fetchActiveOrders(true)
      return orderId
    } catch (err) {
      if (requestGeneration !== generation) throw err
      // 保留后端真实错误信息（ApiError.message），供调用方展示。
      error.value = (err as Error)?.message || '提交订单失败'
      console.error('Failed to submit order:', err)
      throw err
    }
  }

  function reset() {
    generation += 1
    activeOrders.value = []
    recentOrders.value = []
    lastFetched.value = 0
    loading.value = false
    error.value = null
  }

  return {
    activeOrders,
    recentOrders,
    loading,
    error,
    isStale,
    orderCount,
    pendingOrders,
    filledOrders,
    fetchActiveOrders,
    fetchRecentOrders,
    placeOrder,
    reset,
  }
})
