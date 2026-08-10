import { invoke } from '@tauri-apps/api/core'
import type { Alert, LogEntry } from './types'

/**
 * Monitoring service.
 *
 * Read Prometheus metrics, alerts, and application logs.
 * Acknowledge alerts to clear them from the active list.
 */

export function getMetrics(): Promise<Record<string, number>> {
  return invoke<Record<string, number>>('get_metrics')
}

export function getAlerts(): Promise<Alert[]> {
  return invoke<Alert[]>('get_alerts')
}

export function acknowledgeAlert(alertId: number): Promise<boolean> {
  return invoke<boolean>('acknowledge_alert', { alertId: alertId.toString() })
}

export function getLogs(
  level?: string,
  limit?: number,
): Promise<LogEntry[]> {
  return invoke<LogEntry[]>('get_logs', { level, limit })
}

export function checkRedisStatus(): Promise<boolean> {
  return invoke<boolean>('check_redis_status')
}
