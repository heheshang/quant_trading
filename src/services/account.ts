import { invoke } from '@tauri-apps/api/core'
import type { AccountInfo, Position } from './types'

/**
 * Account & position service.
 *
 * Read-only views of the trading account and current positions.
 */

export function getAccountInfo(): Promise<AccountInfo> {
  return invoke<AccountInfo>('get_account_info')
}

export function getPositions(): Promise<Position[]> {
  return invoke<Position[]>('get_positions')
}
