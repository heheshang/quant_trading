import { load, type Store } from '@tauri-apps/plugin-store'

const STORE_FILE = 'auth.json'
let storePromise: Promise<Store | null> | null = null

/**
 * 安全存储适配器：优先用 Tauri `plugin-store`（文件存储，WebView 无法直接读），
 * 非 Tauri/测试环境回退 localStorage。
 */
async function getStore(): Promise<Store | null> {
  if (!storePromise) {
    storePromise = load(STORE_FILE, { autoSave: true }).catch(() => null)
  }
  return storePromise
}

export async function getItem(key: string): Promise<string | null> {
  const store = await getStore()
  if (store) {
    const v = await store.get<string>(key)
    return v ?? null
  }
  return localStorage.getItem(key)
}

export async function setItem(key: string, value: string): Promise<void> {
  const store = await getStore()
  if (store) {
    await store.set(key, value)
    await store.save()
    return
  }
  localStorage.setItem(key, value)
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
