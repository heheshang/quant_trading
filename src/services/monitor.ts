import { call } from './transport'
import type { ThresholdConfig as MonitorThresholds } from '@/components/monitor/types'
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

const THRESHOLDS_STORAGE_KEY = 'monitor_thresholds'

const DEFAULT_THRESHOLDS: MonitorThresholds = {
  maxDrawdown: 20,
  dailyLoss: 10,
  concentration: 50,
  leverage: 3,
  orderLatency: 1000,
  varWarning: 5,
}

/**
 * Read persisted monitoring-alert thresholds from localStorage.
 *
 * The backend exposes no command for these UI-only alert thresholds today, so
 * they are persisted client-side and restored here (survives a refresh). Fields
 * missing from storage fall back to the defaults.
 */
export function getThresholds(): MonitorThresholds {
  try {
    const raw = localStorage.getItem(THRESHOLDS_STORAGE_KEY)
    if (!raw) return { ...DEFAULT_THRESHOLDS }
    const parsed: unknown = JSON.parse(raw)
    return { ...DEFAULT_THRESHOLDS, ...(parsed as Partial<MonitorThresholds>) }
  } catch {
    return { ...DEFAULT_THRESHOLDS }
  }
}

/** Persist monitoring-alert thresholds to localStorage (best-effort). */
export function saveThresholds(cfg: MonitorThresholds): void {
  try {
    localStorage.setItem(THRESHOLDS_STORAGE_KEY, JSON.stringify(cfg))
  } catch {
    // Ignore storage errors (private mode / quota)
  }
}
