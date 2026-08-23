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

export function getActiveOrders(): Promise<Order[]> {
  return call<Order[]>('get_active_orders')
}

/**
 * Cancel an in-progress order (paper / OrderManager), identified by its
 * internal `order_id`.
 */
export function cancelOrder(orderId: number): Promise<boolean> {
  return call<boolean>('cancel_order', { orderId })
}
