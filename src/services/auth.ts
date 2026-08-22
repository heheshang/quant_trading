import { call } from './transport'

/**
 * Authentication / user profile service.
 *
 * Login, token verification, password change, and user profile retrieval/update.
 */

export function login(username: string, password: string): Promise<string> {
  return call<string>('login', { username, password })
}

export function verifyToken(token: string): Promise<boolean> {
  return call<boolean>('verify_token', { token })
}

export function updateProfile(profileData: Record<string, unknown>): Promise<boolean> {
  return call<boolean>('update_profile', { profileData })
}

export function changePassword(
  currentPassword: string,
  newPassword: string,
  username?: string,
): Promise<boolean> {
  return call<boolean>('change_password', {
    currentPassword,
    newPassword,
    username,
  })
}

export function getUserProfile(username?: string): Promise<Record<string, unknown>> {
  return call<Record<string, unknown>>('get_user_profile', { username })
}
