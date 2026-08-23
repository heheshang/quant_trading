import { describe, it, expect, beforeEach } from 'vitest'
import { resetTauriMocks, mockInvoke } from './mock-tauri'
import { enable2FA, verify2FACode, disable2FA } from '../services/twoFA'
import { getAuditLogs } from '../services/audit'
import { optimizeStrategy } from '../services/optimizer'

describe('consistency services contract', () => {
  beforeEach(() => resetTauriMocks())

  it('enable2FA invokes enable_2fa with camelCase userId', async () => {
    mockInvoke.mockResolvedValueOnce({
      secret: 'ABCDEF',
      encrypted_secret: 'enc',
      otpauth_uri: 'otpauth://totp/test',
    })
    const res = await enable2FA(7)
    expect(res.secret).toBe('ABCDEF')
    expect(mockInvoke).toHaveBeenCalledWith('enable_2fa', { userId: 7 })
  })

  it('verify2FACode invokes verify_2fa_code with userId + code', async () => {
    mockInvoke.mockResolvedValueOnce(true)
    const ok = await verify2FACode(7, '123456')
    expect(ok).toBe(true)
    expect(mockInvoke).toHaveBeenCalledWith('verify_2fa_code', { userId: 7, code: '123456' })
  })

  it('disable2FA invokes disable_2fa with userId + code', async () => {
    mockInvoke.mockResolvedValueOnce(true)
    const ok = await disable2FA(7, '654321')
    expect(ok).toBe(true)
    expect(mockInvoke).toHaveBeenCalledWith('disable_2fa', { userId: 7, code: '654321' })
  })

  it('getAuditLogs passes camelCase optional args', async () => {
    mockInvoke.mockResolvedValueOnce([])
    await getAuditLogs({ userId: 3, action: 'Login', limit: 20, offset: 40 })
    expect(mockInvoke).toHaveBeenCalledWith('get_audit_logs', {
      userId: 3,
      action: 'Login',
      limit: 20,
      offset: 40,
    })
  })

  it('getAuditLogs omits undefined optional args', async () => {
    mockInvoke.mockResolvedValueOnce([])
    await getAuditLogs({})
    expect(mockInvoke).toHaveBeenCalledWith('get_audit_logs', {})
  })

  it('optimizeStrategy invokes optimize_strategy with camelCase args', async () => {
    mockInvoke.mockResolvedValueOnce({
      total_combinations: 2,
      combinations_returned: 2,
      top_n_requested: 5,
      combinations: [],
      best: null,
    })
    await optimizeStrategy({
      strategyId: 's1',
      paramGrid: { rsi_period: [14, 7] },
      metric: 'sharpe_ratio',
      topN: 5,
    })
    expect(mockInvoke).toHaveBeenCalledWith('optimize_strategy', {
      strategyId: 's1',
      paramGrid: { rsi_period: [14, 7] },
      metric: 'sharpe_ratio',
      topN: 5,
    })
  })
})
