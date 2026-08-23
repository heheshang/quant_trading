import { call } from './transport'

/**
 * Audit log service.
 *
 * Wraps the `get_audit_logs` Tauri command. `userId` / `action` are optional
 * filters; `limit` / `offset` drive pagination.
 */

export interface AuditLog {
  id: number
  timestamp: string
  user_id: string
  username: string
  action: string
  resource: string
  details: Record<string, unknown>
  ip_address: string | null
  success: boolean
  error_message: string | null
}

export interface AuditLogQuery {
  userId?: number
  action?: string
  limit?: number
  offset?: number
}

export function getAuditLogs(query: AuditLogQuery = {}): Promise<AuditLog[]> {
  const args: Record<string, unknown> = {}
  if (query.userId !== undefined) args.userId = query.userId
  if (query.action !== undefined) args.action = query.action
  if (query.limit !== undefined) args.limit = query.limit
  if (query.offset !== undefined) args.offset = query.offset
  return call<AuditLog[]>('get_audit_logs', args)
}
