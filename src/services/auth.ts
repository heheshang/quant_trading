import { invoke } from '@tauri-apps/api/core'

/**
 * Authentication / user profile service.
 *
 * Login, token verification, password change, and user profile retrieval/update.
 */

export function login(username: string, password: string): Promise<string> {
  return invoke<string>('login', { username, password })
}

export function verifyToken(token: string): Promise<boolean> {
  return invoke<boolean>('verify_token', { token })
}

export function updateProfile(profileData: Record<string, unknown>): Promise<boolean> {
  return invoke<boolean>('update_profile', { profileData })
}

export function changePassword(
  currentPassword: string,
  newPassword: string,
  username?: string,
): Promise<boolean> {
  return invoke<boolean>('change_password', {
    currentPassword,
    newPassword,
    username,
  })
}

export function getUserProfile(username?: string): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_user_profile', { username })
}
