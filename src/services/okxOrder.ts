import { call } from './transport'
import type { OkxOrder, OkxPlaceOrderRequest, Order } from './types'

/**
 * OKX order service (SoC).
 *
 * Order placement / cancellation / execution against the OKX trading API.
 * Account, market and status reads live in `services/okx.ts`.
 */

export function placeOkxOrder(request: OkxPlaceOrderRequest): Promise<OkxOrder> {
  return call<OkxOrder>('place_okx_order', { request })
}

export function cancelOkxOrder(instId: string, ordId: string): Promise<boolean> {
  return call<boolean>('cancel_okx_order', { instId, ordId })
}

export function executeOkxOrder(order: Order): Promise<string> {
  return call<string>('execute_okx_order', { order })
}
