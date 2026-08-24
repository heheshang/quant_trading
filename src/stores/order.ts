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
  let lastFetched = 0

  const isStale = computed(() => Date.now() - lastFetched > CACHE_TTL_MS)

  const orderCount = computed(() => activeOrders.value.length)

  const pendingOrders = computed(() =>
    activeOrders.value.filter((o: Order) => o.status === 'Pending' || o.status === 'Submitted'),
  )

  const filledOrders = computed(() =>
    activeOrders.value.filter((o: Order) => o.status === 'Filled' || o.status === 'PartiallyFilled'),
  )

  async function fetchActiveOrders(force = false) {
    if (!force && activeOrders.value.length > 0 && !isStale.value) return
    loading.value = true
    error.value = null
    try {
      activeOrders.value = await getActiveOrders()
      lastFetched = Date.now()
    } catch (err) {
      error.value = '获取订单信息失败'
      console.error('Failed to fetch active orders:', err)
    } finally {
      loading.value = false
    }
  }

  async function fetchRecentOrders(force = false) {
    if (!force && recentOrders.value.length > 0 && !isStale.value) return
    try {
      recentOrders.value = await getRecentOrders(50)
    } catch {
      // 忽略：历史拉取失败不阻塞页面
    }
  }

  async function placeOrder(order: Order): Promise<string | null> {
    try {
      const orderId = await submitOrder(order)
      await fetchActiveOrders(true)
      return orderId
    } catch (err) {
      error.value = '提交订单失败'
      console.error('Failed to submit order:', err)
      return null
    }
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
  }
})
