// Vitest setup: mock Tauri APIs and browser APIs not available in jsdom

import { vi } from 'vitest'

function createMemoryStorage(): Storage {
  const store = new Map<string, string>()
  return {
    get length() {
      return store.size
    },
    clear() {
      store.clear()
    },
    getItem(key: string) {
      return store.get(key) ?? null
    },
    key(index: number) {
      return Array.from(store.keys())[index] ?? null
    },
    removeItem(key: string) {
      store.delete(key)
    },
    setItem(key: string, value: string) {
      store.set(key, String(value))
    },
  } as Storage
}

// Vitest 4 + jsdom 29 在部分环境中不会把 localStorage/sessionStorage 暴露到
// globalThis；测试代码同时使用 global 与 window 两种访问方式，因此统一注入。
const globalWithStorage = globalThis as typeof globalThis & {
  localStorage?: Storage
  window?: { localStorage?: Storage }
}
const localStorage = globalWithStorage.localStorage ?? globalWithStorage.window?.localStorage ?? createMemoryStorage()
Object.defineProperty(globalThis, 'localStorage', {
  value: localStorage,
  configurable: true,
})
Object.defineProperty(globalThis, 'sessionStorage', {
  value: createMemoryStorage(),
  configurable: true,
})

// Mock @tauri-apps/api/event
const mockUnlisten = vi.fn()
const mockListen = vi.fn().mockResolvedValue(mockUnlisten)

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
}))

// Mock @tauri-apps/api/core
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}))

// Mock ResizeObserver (not available in jsdom)
// Must be a real class/constructor — @vueuse/core calls `new ResizeObserver()`
class MockResizeObserver {
  observe = vi.fn()
  unobserve = vi.fn()
  disconnect = vi.fn()
}
globalThis.ResizeObserver = MockResizeObserver as unknown as typeof ResizeObserver

// Mock echarts — 'echarts' and 'echarts/core' share a single mock instance
// so assertions on either entry point observe the same chart object.
const mockEChartsInstance = {
  setOption: vi.fn(),
  dispose: vi.fn(),
  getInstanceByDom: vi.fn(),
  on: vi.fn(),
  off: vi.fn(),
  resize: vi.fn(),
  clear: vi.fn(),
}
const mockEChartsInit = vi.fn().mockReturnValue(mockEChartsInstance)
const echartsMockFactory = () => ({
  init: mockEChartsInit,
  getInstanceByDom: vi.fn().mockReturnValue(mockEChartsInstance),
  use: vi.fn(),
  default: { init: mockEChartsInit },
})
vi.mock('echarts', () => echartsMockFactory())
vi.mock('echarts/core', () => echartsMockFactory())

// Mock Element Plus
vi.mock('element-plus', () => ({
  default: {},
  ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
  ElNotification: { success: vi.fn(), error: vi.fn() },
  ElMessageBox: { confirm: vi.fn() },
}))

// Mock CSS imports (Element Plus CSS, etc.)
vi.mock('element-plus/theme-chalk/base.css', () => ({}))
vi.mock('element-plus/theme-chalk/el-radio-group.css', () => ({}))
vi.mock('element-plus/theme-chalk/el-radio-button.css', () => ({}))

export { mockListen, mockUnlisten }
