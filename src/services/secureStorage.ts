import { invoke } from '@tauri-apps/api/core'
import { load, type Store } from '@tauri-apps/plugin-store'

const STORE_FILE = 'auth.json'
let storePromise: Promise<Store | null> | null = null

/**
 * 安全存储适配器：优先用 Tauri `plugin-store`（文件存储，WebView 无法直接读），
 * 非 Tauri/测试环境回退 localStorage。
 *
 * 值经后端 `secure_encrypt`/`secure_decrypt`（AES-GCM，用后端 ENCRYPTION_KEY）加密后落盘，
 * 即使存储文件泄露也不暴露明文 token。
 */
async function getStore(): Promise<Store | null> {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { autoSave: true }).catch(() => null)
  }
  return storePromise
}

async function enc(v: string): Promise<string> {
  // 非 Tauri/测试（invoke 不可用）回退明文。
  try {
    return await invoke<string>('secure_encrypt', { value: v })
  } catch {
    return v
  }
}

async function dec(v: string): Promise<string> {
  try {
    return await invoke<string>('secure_decrypt', { value: v })
  } catch {
    return v
  }
}

export async function getItem(key: string): Promise<string | null> {
  const store = await getStore()
  const raw = store ? await store.get<string>(key) : localStorage.getItem(key)
  if (raw === null || raw === undefined) return null
  return dec(raw)
}

export async function setItem(key: string, value: string): Promise<void> {
  const cipher = await enc(value)
  const store = await getStore()
  if (store) {
    await store.set(key, cipher)
    await store.save()
    return
  }
  localStorage.setItem(key, cipher)
}

export async function removeItem(key: string): Promise<void> {
  const store = await getStore()
  if (store) {
    await store.delete(key)
    await store.save()
    return
  }
  localStorage.removeItem(key)
}

/** 是否真正使用 Tauri store（而非回退 localStorage）。 */
export async function usingSecureStore(): Promise<boolean> {
  return (await getStore()) !== null
}
