import { call } from './transport'
import type { Order } from './types'

/**
 * Order service.
 *
 * Submit orders and query the active order book.
 */

export function submitOrder(order: Order): Promise<string> {
  return call<string>('submit_order', { order })
}

export function getActiveOrders(exchange?: string): Promise<Order[]> {
  return call<Order[]>('get_active_orders', { exchange: exchange || null })
}

/** 最近订单（含已成交/撤单/拒绝），按时间倒序；可按种类(paper/live/algorithm)过滤。 */
export function getRecentOrders(limit = 50, exchange?: string): Promise<Order[]> {
  return call<Order[]>('get_recent_orders', { limit, exchange: exchange || null })
}

/**
 * Cancel an in-progress order (paper / OrderManager), identified by its
 * internal `order_id`.
 */
export function cancelOrder(orderId: number): Promise<boolean> {
  return call<boolean>('cancel_order', { orderId })
}
