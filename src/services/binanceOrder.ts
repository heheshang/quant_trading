import { call } from './transport'
import type { BinanceOrder, BinancePlaceOrderRequest } from './types'

/**
 * Binance order service (SoC).
 *
 * Order placement / cancellation against the Binance REST API.
 * Account + market reads live in `services/binance.ts`.
 */

export function placeBinanceOrder(request: BinancePlaceOrderRequest): Promise<BinanceOrder> {
  return call<BinanceOrder>('place_binance_order', { request })
}

export function cancelBinanceOrder(symbol: string, orderId: number): Promise<void> {
  return call<void>('cancel_binance_order', { symbol, orderId })
}
