import { invoke } from '@tauri-apps/api/core'
import type {
  OkxBalance,
  OkxCandle,
  OkxInstrument,
  OkxOrder,
  OkxPlaceOrderRequest,
  OkxPosition,
  Order,
} from './types'

/**
 * OKX exchange service.
 *
 * Read account balances/positions, place/cancel orders, fetch candles and instruments,
 * check exchange status and announcements, and execute orders against the OKX trading API.
 */

export function getOkxBalance(ccy?: string): Promise<OkxBalance[]> {
  return invoke<OkxBalance[]>('get_okx_balance', { ccy })
}

export function getOkxPositions(instId?: string): Promise<OkxPosition[]> {
  return invoke<OkxPosition[]>('get_okx_positions', { instId })
}

export function placeOkxOrder(
  request: OkxPlaceOrderRequest,
): Promise<OkxOrder> {
  return invoke<OkxOrder>('place_okx_order', { request })
}

export function cancelOkxOrder(
  instId: string,
  ordId: string,
): Promise<boolean> {
  return invoke<boolean>('cancel_okx_order', { instId, ordId })
}

export function getOkxCandles(
  instId: string,
  bar?: string,
  limit?: number,
): Promise<OkxCandle[]> {
  return invoke<OkxCandle[]>('get_okx_candles', { instId, bar, limit })
}

export function getOkxInstruments(
  instType?: string,
): Promise<OkxInstrument[]> {
  return invoke<OkxInstrument[]>('get_okx_instruments', { instType })
}

export function checkOkxStatus(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('check_okx_status')
}

export function getOkxAnnouncements(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_okx_announcements')
}

export function executeOkxOrder(order: Order): Promise<string> {
  return invoke<string>('execute_okx_order', { order })
}
