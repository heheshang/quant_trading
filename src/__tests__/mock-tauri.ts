import { vi } from 'vitest'
import { invoke, type InvokeArgs } from '@tauri-apps/api/core'

// The mocked invoke function — re-exported so existing tests can use it directly
const mockInvoke = vi.mocked(invoke)

/**
 * Set up a mock return value for a specific Tauri command.
 * Only the last call to mockTauriInvoke per command name takes effect
 * (subsequent calls overwrite the implementation). Use mockTauriInvokeMap
 * to mock several commands at once.
 *
 * @example
 * mockTauriInvoke('get_okx_balance', [{ ccy: 'BTC', avail_bal: 1.5, frozen_bal: 0.5, eq: 2.0 }])
 */
export function mockTauriInvoke<T>(command: string, returnValue: T): void {
  mockInvoke.mockImplementation(
    async (cmd: string, _args?: InvokeArgs) => {
      if (cmd === command) return returnValue
      return undefined
    },
  )
}

/**
 * Set up a mock that throws for a specific Tauri command.
 *
 * @example
 * mockTauriInvokeError('get_okx_balance', 'Network error')
 */
export function mockTauriInvokeError(
  command: string,
  errorMessage: string,
): void {
  mockInvoke.mockImplementation(
    async (cmd: string, _args?: InvokeArgs) => {
      if (cmd === command) throw new Error(errorMessage)
      return undefined
    },
  )
}

/**
 * Create a multi-command mock that returns different values per command name.
 * Commands not in the map return undefined.
 *
 * @example
 * mockTauriInvokeMap({
 *   get_okx_balance: [{ ccy: 'BTC', avail_bal: 1.5, frozen_bal: 0.5, eq: 2.0 }],
 *   check_okx_status: { connected: true, demo_trading: true },
 * })
 */
export function mockTauriInvokeMap(
  map: Record<string, unknown>,
): void {
  mockInvoke.mockImplementation(
    async (cmd: string, _args?: InvokeArgs) => {
      if (cmd in map) return map[cmd]
      return undefined
    },
  )
}

/**
 * Reset all mocks to the default behaviour (resolves with empty object).
 * Clears call history and implementation stubs.
 */
export function resetTauriMocks(): void {
  mockInvoke.mockReset()
  mockInvoke.mockResolvedValue({})
}

export { mockInvoke }
