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
