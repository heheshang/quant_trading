import { shallowRef } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { ConnectionStatusEvent, WsConnectionStatus } from '@/services/types'

// --- Module-level singleton state ---
const status = shallowRef<WsConnectionStatus>('disconnected')
const retryIn = shallowRef<number>(0)

let unlisteners: UnlistenFn[] = []
let isListening = false

async function startListening() {
  if (isListening) return
  isListening = true

  const unlisten = await listen<ConnectionStatusEvent>('ws:connection_status', (event) => {
    const { status: newStatus, retry_in } = event.payload
    status.value = newStatus
    retryIn.value = retry_in ?? 0
  })
  unlisteners.push(unlisten)
}

function cleanup() {
  for (const unlisten of unlisteners) {
    unlisten()
  }
  unlisteners = []
  isListening = false
}

export function useWebSocketStatus() {
  return {
    status,
    retryIn,
    startListening,
    cleanup,
  }
}
