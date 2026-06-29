import { invoke } from '@tauri-apps/api/core'
import type { Order } from './types'

/**
 * Order service.
 *
 * Submit orders and query the active order book.
 */

export function submitOrder(order: Order): Promise<string> {
  return invoke<string>('submit_order', { order })
}

export function getActiveOrders(): Promise<Order[]> {
  return invoke<Order[]>('get_active_orders')
}
