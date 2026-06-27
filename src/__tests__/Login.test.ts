import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Login from '@/views/Login.vue'
import { invoke } from '@tauri-apps/api/core'

const mockRouterPush = vi.fn()

vi.mock('vue-router', () => ({
  useRouter: () => ({ push: mockRouterPush }),
}))

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

/** Mount Login and wait for onMounted to settle. */
async function mountLogin(): Promise<any> {
  const wrapper = mount(Login, { global: { plugins: [ElementPlus] } })
  await wrapper.vm.$nextTick()
  await wrapper.vm.$nextTick()
  return wrapper
}

describe('Login.vue - 按钮测试', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    mockRouterPush.mockReset()
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'login') return 'test-token-123'
      if (cmd === 'verify_token') return true
      return {}
    })
  })

  it('登录 - 调用 login API', async () => {
    const wrapper = await mountLogin()
    wrapper.vm.loginForm.username = 'admin'
    wrapper.vm.loginForm.password = 'admin123'

    await wrapper.vm.handleLogin()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('login', {
      username: 'admin',
      password: 'admin123',
    })
  })

  it('登录成功后 - 跳转到仪表盘', async () => {
    const wrapper = await mountLogin()
    wrapper.vm.loginForm.username = 'admin'
    wrapper.vm.loginForm.password = 'admin123'

    await wrapper.vm.handleLogin()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('verify_token', { token: 'test-token-123' })
    expect(mockRouterPush).toHaveBeenCalledWith('/dashboard')
  })

  it('登录失败时 - 不跳转', async () => {
    mockInvoke.mockRejectedValue(new Error('用户名或密码错误'))

    const wrapper = await mountLogin()
    wrapper.vm.loginForm.username = 'baduser'
    wrapper.vm.loginForm.password = 'badpass'

    await wrapper.vm.handleLogin()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(mockRouterPush).not.toHaveBeenCalled()
    expect(wrapper.exists()).toBe(true)
  })

  it('记住我 - 保存凭据到 localStorage', async () => {
    const wrapper = await mountLogin()
    wrapper.vm.loginForm.username = 'admin'
    wrapper.vm.loginForm.password = 'admin123'
    wrapper.vm.loginForm.remember = true

    await wrapper.vm.handleLogin()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(localStorage.getItem('remembered_username')).toBe('admin')
  })

  it('loading 状态 - 提交期间为 true', async () => {
    // Ensure form validation passes by providing valid form ref
    const wrapper = await mountLogin()
    // Set up a form ref mock that passes validation
    wrapper.vm.loginFormRef = { validate: vi.fn((cb: any) => cb(true)) } as any

    // Then make invoke calls hang
    mockInvoke.mockImplementation(() => new Promise(() => {}))

    wrapper.vm.loginForm.username = 'admin'
    wrapper.vm.loginForm.password = 'admin123'

    wrapper.vm.handleLogin()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.loading).toBe(true)
  })

  it('表单验证失败 - 不调用 login API', async () => {
    const wrapper = await mountLogin()
    // Mock form validation to fail
    wrapper.vm.loginFormRef = { validate: vi.fn((cb: any) => cb(false)) } as any

    await wrapper.vm.handleLogin()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    const loginCalls = mockInvoke.mock.calls.filter(([cmd]) => cmd === 'login')
    expect(loginCalls.length).toBe(0)
  })

  it('已认证用户自动跳转', async () => {
    localStorage.setItem('isAuthenticated', 'true')
    mount(Login, { global: { plugins: [ElementPlus] } })
    await new Promise(r => setTimeout(r, 100))
    expect(mockRouterPush).toHaveBeenCalledWith('/dashboard')
  })
})
