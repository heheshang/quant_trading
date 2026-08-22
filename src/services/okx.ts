import { call } from './transport'
import type {
  OkxBalance,
  OkxCandle,
  OkxAnnouncementPage,
  OkxConnectionStatus,
  OkxInstrument,
  OkxPosition,
} from './types'

/**
 * OKX exchange service (SoC).
 *
 * Account balance/position + market/status/announcement reads.
 * Order placement/cancellation/execution lives in `services/okxOrder.ts`.
 */

export function getOkxBalance(ccy?: string): Promise<OkxBalance[]> {
  return call<OkxBalance[]>('get_okx_balance', { ccy })
}

export function getOkxPositions(instId?: string): Promise<OkxPosition[]> {
  return call<OkxPosition[]>('get_okx_positions', { instId })
}

export function getOkxCandles(
  instId: string,
  bar?: string,
  limit?: number,
): Promise<OkxCandle[]> {
  return call<OkxCandle[]>('get_okx_candles', { instId, bar, limit })
}

export function getOkxInstruments(instType?: string): Promise<OkxInstrument[]> {
  return call<OkxInstrument[]>('get_okx_instruments', { instType })
}

export function checkOkxStatus(): Promise<OkxConnectionStatus> {
  return call<OkxConnectionStatus>('check_okx_status')
}

export function getOkxAnnouncements(): Promise<OkxAnnouncementPage[]> {
  return call<OkxAnnouncementPage[]>('get_okx_announcements')
}
