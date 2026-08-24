import { call } from './transport'
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
  // call<T> 解包信封：成功返回后端 `data` 字符串（即密文）。
  // 仅接受字符串结果，否则（非 Tauri/测试、后端不可用）回退明文，保证可用性。
  try {
    const r = await call<string>('secure_encrypt', { value: v })
    return typeof r === 'string' && r.length > 0 ? r : v
  } catch {
    return v
  }
}

async function dec(v: string): Promise<string> {
  // `v` 是落盘的密文；call<T> 解包信封返回解密后的明文。
  // 仅接受字符串结果；若 `v` 是历史明文（旧数据）或后端不可用，解密失败即回退原值。
  try {
    const r = await call<string>('secure_decrypt', { value: v })
    return typeof r === 'string' && r.length > 0 ? r : v
  } catch {
    return v
  }
}

/**
 * 判断值是否「不是有效密文」（即迁移前的明文）。
 *
 * 密文 = base64(nonce(12) + ciphertext + tag)，解码后必然 ≥12 字节。
 * 明文 JWT（含 `.`）或任意非 base64 字符串会抛 `InvalidCharacterError`。
 * 仅当确认为明文时才做「加密写回」迁移——避免把真密文（如 key 变更解不开）
 * 误当明文二次加密而损坏。
 */
function looksLikePlaintext(v: string): boolean {
  if (typeof atob !== 'function') return false // 无法判断 → 走正常解密（失败会回退原值）
  try {
    return atob(v).length < 12
  } catch {
    return true
  }
}

export async function getItem(key: string): Promise<string | null> {
  const store = await getStore()
  const raw = store ? await store.get<string>(key) : localStorage.getItem(key)
  if (raw === null || raw === undefined) return null
  // 迁移：旧代码（enc 未加密时）写入的明文 token → 加密写回，动态清除明文残留。
  if (looksLikePlaintext(raw)) {
    await setItem(key, raw)
    return raw
  }
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
