import { call } from './transport'
import type { Alert, LogEntry } from './types'

/**
 * Monitoring service.
 *
 * Read Prometheus metrics, alerts, and application logs.
 * Acknowledge alerts to clear them from the active list.
 */

export function getMetrics(): Promise<Record<string, number>> {
  return call<Record<string, number>>('get_metrics')
}

export function getAlerts(): Promise<Alert[]> {
  return call<Alert[]>('get_alerts')
}

export function acknowledgeAlert(alertId: number): Promise<boolean> {
  return call<boolean>('acknowledge_alert', { alertId: alertId.toString() })
}

export function getLogs(
  level?: string,
  limit?: number,
): Promise<LogEntry[]> {
  return call<LogEntry[]>('get_logs', { level, limit })
}

export function checkRedisStatus(): Promise<boolean> {
  return call<boolean>('check_redis_status')
}
