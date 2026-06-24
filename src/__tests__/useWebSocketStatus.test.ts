import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mockListen, mockUnlisten } from './setup'
import type { useWebSocketStatus as UseWebSocketStatusFn } from '../composables/useWebSocketStatus'

describe('useWebSocketStatus', () => {
  let useWebSocketStatus: typeof UseWebSocketStatusFn

  beforeEach(async () => {
    vi.clearAllMocks()
    vi.resetModules()
    const mod = await import('../composables/useWebSocketStatus')
    useWebSocketStatus = mod.useWebSocketStatus
  })

  it('returns the expected shape (status, retryIn, startListening, cleanup)', () => {
    const result = useWebSocketStatus()

    expect(result).toHaveProperty('status')
    expect(result).toHaveProperty('retryIn')
    expect(result).toHaveProperty('startListening')
    expect(result).toHaveProperty('cleanup')
    expect(typeof result.startListening).toBe('function')
    expect(typeof result.cleanup).toBe('function')
  })

  it('has initial status "disconnected" and retryIn 0', () => {
    const { status, retryIn } = useWebSocketStatus()

    expect(status.value).toBe('disconnected')
    expect(retryIn.value).toBe(0)
  })

  it('startListening() registers 1 WS listener for ws:connection_status', async () => {
    const { startListening } = useWebSocketStatus()

    await startListening()

    expect(mockListen).toHaveBeenCalledTimes(1)
    expect(mockListen).toHaveBeenCalledWith('ws:connection_status', expect.any(Function))
  })

  it('startListening() is idempotent', async () => {
    const { startListening } = useWebSocketStatus()

    await startListening()
    await startListening()
    await startListening()

    expect(mockListen).toHaveBeenCalledTimes(1)
  })

  it('receiving "connected" status updates the status ref', async () => {
    const { status, startListening } = useWebSocketStatus()

    await startListening()

    const listenerCallback = mockListen.mock.calls[0][1]
    listenerCallback({ payload: { status: 'connected' } })

    expect(status.value).toBe('connected')
  })

  it('receiving "reconnecting" status updates the ref', async () => {
    const { status, startListening } = useWebSocketStatus()

    await startListening()

    const listenerCallback = mockListen.mock.calls[0][1]
    listenerCallback({ payload: { status: 'reconnecting' } })

    expect(status.value).toBe('reconnecting')
  })

  it('receiving "disconnected" status with retry_in updates both refs', async () => {
    const { status, retryIn, startListening } = useWebSocketStatus()

    await startListening()

    const listenerCallback = mockListen.mock.calls[0][1]
    listenerCallback({ payload: { status: 'disconnected', retry_in: 5 } })

    expect(status.value).toBe('disconnected')
    expect(retryIn.value).toBe(5)
  })

  it('cleanup() unregisters the listener', async () => {
    const { startListening, cleanup } = useWebSocketStatus()

    await startListening()

    cleanup()

    expect(mockUnlisten).toHaveBeenCalledTimes(1)
  })
})
