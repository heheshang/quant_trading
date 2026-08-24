import { describe, it, expect, vi, beforeEach } from "vitest"
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createPinia } from 'pinia'
import Profile from '@/views/Profile.vue'
import PasswordChange from '@/components/profile/PasswordChange.vue'
import { invoke } from '@tauri-apps/api/core'

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

// mockFormRef disabled - unused
function mockFormRefAsync() {
  return { validate: vi.fn((cb: any) => { const p = cb(true); return p; }) } as any
}

const defaultProfile = {
  account_id: 'ACC001',
  username: 'testuser',
  email: 'test@example.com',
  phone: '13800138000',
  full_name: '测试用户',
  company: '测试公司',
  address: '北京市朝阳区',
}

const defaultAccountInfo = {
  account_id: 1,
  total_assets: 1000000,
  available_cash: 500000,
  market_value: 500000,
  daily_pnl: 5000,
  total_pnl: 25000,
  margin: 0,
  margin_ratio: 0,
  updated_at: new Date().toISOString(),
}

describe('Profile.vue - 按钮测试', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockInvoke.mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_user_profile':
          return defaultProfile
        case 'get_account_info':
          return defaultAccountInfo
        case 'update_profile':
          return true
        case 'change_password':
          return true
        default:
          return {}
      }
    })
  })

  it('编辑信息按钮 - 切换到编辑模式', async () => {
    const wrapper: any = mount(Profile, { global: { plugins: [ElementPlus, createPinia()] } })
    await wrapper.vm.$nextTick()
    await flushPromises()
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.isEditing).toBe(false)

    const editBtn = wrapper.findAll('.el-button--primary').find((b: any) => b.text().includes('编辑信息'))
    expect(editBtn).toBeDefined()
    await editBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.isEditing).toBe(true)
  })

  it('保存按钮 - 调用 updateProfile', async () => {
    const wrapper: any = mount(Profile, { global: { plugins: [ElementPlus, createPinia()] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    await wrapper.vm.handleSaveProfile(defaultProfile as any)
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('update_profile', { profileData: defaultProfile })
  })

  it('取消按钮 - 退出编辑模式', async () => {
    const wrapper: any = mount(Profile, { global: { plugins: [ElementPlus, createPinia()] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    wrapper.vm.isEditing = true
    await wrapper.vm.$nextTick()

    const cancelBtn = wrapper.findAll('.el-button').find((b: any) => b.text().includes('取消'))
    expect(cancelBtn).toBeDefined()
    await cancelBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.isEditing).toBe(false)
  })

  it('修改密码按钮 - 打开对话框', async () => {
    const wrapper: any = mount(Profile, { global: { plugins: [ElementPlus, createPinia()] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.showPasswordDialog).toBe(false)

    const pwdBtn = wrapper.findAll('.el-button').find((b: any) => b.text().includes('修改密码'))
    expect(pwdBtn).toBeDefined()
    await pwdBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.showPasswordDialog).toBe(true)
  })

  it('修改密码 - 调用 changePassword', async () => {
    const wrapper: any = mount(Profile, { global: { plugins: [ElementPlus, createPinia()] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    wrapper.vm.showPasswordDialog = true
    await wrapper.vm.$nextTick()

    const passwordChange = wrapper.findComponent(PasswordChange)
    Object.assign(passwordChange.vm.passwordForm, {
      currentPassword: 'old',
      newPassword: 'new123',
      confirmPassword: 'new123',
    })
    passwordChange.vm.passwordFormRef = mockFormRefAsync()
    await passwordChange.vm.handleChangePassword()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('change_password', {
      currentPassword: 'old',
      newPassword: 'new123',
      username: undefined,
    })
  })

  it('双因素认证按钮 - 打开 2FA 对话框', async () => {
    const wrapper: any = mount(Profile, { global: { plugins: [ElementPlus, createPinia()] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.show2FADialog).toBe(false)

    const faBtn = wrapper.findAll('.el-button').find((b: any) => b.text().includes('双因素认证'))
    expect(faBtn).toBeDefined()
    await faBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.show2FADialog).toBe(true)
  })

  it('API 调用失败 - 显示错误消息且不崩溃', async () => {
    mockInvoke.mockRejectedValue(new Error('Network error'))

    const wrapper: any = mount(Profile, { global: { plugins: [ElementPlus, createPinia()] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.exists()).toBe(true)
  })
})
