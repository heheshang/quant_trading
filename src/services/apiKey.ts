import { call } from './transport'

/**
 * Exchange API-key management service.
 *
 * Saves exchange credentials (encrypted server-side, secret never returned) and
 * lists previously saved keys in masked form. Backed by the backend
 * `save_api_key` / `get_api_keys` commands (see `src-tauri/src/commands/api_keys.rs`).
 */

export interface MaskedApiKey {
  id: number
  exchange: string
  api_key: string
  passphrase: string | null
  is_active: boolean
}

export interface SaveApiKeyParams {
  user_id: number
  exchange: string
  api_key: string
  secret: string
  passphrase: string | null
}

export function saveApiKey(params: SaveApiKeyParams): Promise<boolean> {
  return call<boolean>('save_api_key', {
    userId: params.user_id,
    exchange: params.exchange,
    apiKey: params.api_key,
    secret: params.secret,
    passphrase: params.passphrase,
  })
}

export function getApiKeys(user_id: number): Promise<MaskedApiKey[]> {
  return call<MaskedApiKey[]>('get_api_keys', { userId: user_id })
}
