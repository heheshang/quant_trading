// Vitest setup: mock Tauri APIs and browser APIs not available in jsdom

import { vi } from 'vitest'

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
global.ResizeObserver = MockResizeObserver as unknown as typeof ResizeObserver

// Mock echarts
vi.mock('echarts', () => {
  const mockECharts = {
    setOption: vi.fn(),
    dispose: vi.fn(),
    getInstanceByDom: vi.fn(),
    on: vi.fn(),
    off: vi.fn(),
    resize: vi.fn(),
    clear: vi.fn(),
  }
  return {
    init: vi.fn().mockReturnValue(mockECharts),
    getInstanceByDom: vi.fn().mockReturnValue(mockECharts),
    default: { init: vi.fn().mockReturnValue(mockECharts) },
  }
})

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
