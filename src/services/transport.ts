import { invoke } from '@tauri-apps/api/core'

/**
 * Single IPC transport (DIP).
 *
 * All frontend services call the Tauri command layer through this function
 * instead of importing `@tauri-apps/api/core` directly. This isolates the
 * framework dependency to one place, so services depend on the abstraction
 * and a future transport change (e.g. HTTP) touches only this module.
 */

type Args = Record<string, unknown>

export function call<T>(cmd: string, args?: Args): Promise<T> {
  // Omit `args` entirely when absent so the IPC call stays arity-identical to
  // a direct `invoke(cmd)` (assertion-safe and avoids `undefined` payloads).
  return args === undefined ? invoke<T>(cmd) : invoke<T>(cmd, args)
}
