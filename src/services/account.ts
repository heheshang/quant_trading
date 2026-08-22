import { call } from './transport'
import type { AccountInfo, Position } from './types'

/**
 * Account & position service.
 *
 * Read-only views of the trading account and current positions.
 */

export function getAccountInfo(): Promise<AccountInfo> {
  return call<AccountInfo>('get_account_info')
}

export function getPositions(): Promise<Position[]> {
  return call<Position[]>('get_positions')
}
