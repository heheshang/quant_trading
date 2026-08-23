import { call } from './transport'

/**
 * Two-factor (TOTP) service.
 *
 * Wraps the `enable_2fa` / `verify_2fa_code` / `disable_2fa` Tauri commands.
 * The enable flow provisions a pending secret on the backend; the returned
 * `secret` / `otpauth_uri` are for the authenticator app. A subsequent
 * `verify2FACode` call with the 6-digit code is what actually enables 2FA.
 */

export interface Enable2faResult {
  secret: string
  encrypted_secret: string
  otpauth_uri: string
}

export function enable2FA(user_id: number): Promise<Enable2faResult> {
  return call<Enable2faResult>('enable_2fa', { userId: user_id })
}

export function verify2FACode(user_id: number, code: string): Promise<boolean> {
  return call<boolean>('verify_2fa_code', { userId: user_id, code })
}

export function disable2FA(user_id: number, code: string): Promise<boolean> {
  return call<boolean>('disable_2fa', { userId: user_id, code })
}
