import { invoke } from '@tauri-apps/api/core'
import type { AccountInfo, Order, Position, RiskConfig } from './types'

/**
 * Risk management service.
 *
 * Read risk metrics and configuration, update risk limits, run pre-trade checks
 * before submitting orders to the exchange.
 */

export function getRiskMetrics(): Promise<Record<string, number>> {
  return invoke<Record<string, number>>('get_risk_metrics')
}

export function getRiskConfig(): Promise<RiskConfig> {
  return invoke<RiskConfig>('get_risk_config')
}

export function updateRiskConfig(config: RiskConfig): Promise<boolean> {
  return invoke<boolean>('update_risk_config', { config })
}

export function preTradeCheck(
  order: Order,
  account: AccountInfo,
  positions: Position[],
): Promise<boolean> {
  return invoke<boolean>('pre_trade_check', { order, account, positions })
}
