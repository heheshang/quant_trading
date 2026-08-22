import { call } from './transport'
import type { AppConfig } from './types'

/**
 * Application configuration service.
 *
 * Read/update the global `AppConfig` that controls database/Redis/trading/risk/OKX/security settings.
 */

export function getConfig(): Promise<AppConfig> {
  return call<AppConfig>('get_config')
}

export function updateConfig(config: AppConfig): Promise<boolean> {
  return call<boolean>('update_config', { config })
}
