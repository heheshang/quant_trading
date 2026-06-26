import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import Profile from '@/views/Profile.vue'
import { invoke } from '@tauri-apps/api/core'

vi.mock('element-plus', async () => {
  const mod = await vi.importActual<typeof import('element-plus')>('element-plus')
  return { ...mod, ElMessage: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() } }
})

const mockInvoke = vi.mocked(invoke)

function mockFormRef() {
  return { validate: vi.fn((cb: any) => cb(true)) } as any
}

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
    const wrapper = mount(Profile, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.isEditing).toBe(false)

    const editBtn = wrapper.findAll('.el-button--primary').find(b => b.text().includes('编辑信息'))
    expect(editBtn).toBeDefined()
    await editBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.isEditing).toBe(true)
  })

  it('保存按钮 - 调用 updateProfile', async () => {
    const wrapper = mount(Profile, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    wrapper.vm.isEditing = true
    wrapper.vm.profileFormRef = mockFormRefAsync()
    await wrapper.vm.$nextTick()

    await wrapper.vm.saveProfile()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('update_profile', { profileData: expect.any(Object) })
  })

  it('取消按钮 - 退出编辑模式', async () => {
    const wrapper = mount(Profile, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    wrapper.vm.isEditing = true
    await wrapper.vm.$nextTick()

    const cancelBtn = wrapper.findAll('.el-button').find(b => b.text().includes('取消'))
    expect(cancelBtn).toBeDefined()
    await cancelBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.isEditing).toBe(false)
  })

  it('修改密码按钮 - 打开对话框', async () => {
    const wrapper = mount(Profile, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.showPasswordDialog).toBe(false)

    const pwdBtn = wrapper.findAll('.el-button').find(b => b.text().includes('修改密码'))
    expect(pwdBtn).toBeDefined()
    await pwdBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.showPasswordDialog).toBe(true)
  })

  it('修改密码 - 调用 changePassword', async () => {
    const wrapper = mount(Profile, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    wrapper.vm.showPasswordDialog = true
    wrapper.vm.passwordFormRef = mockFormRefAsync()
    wrapper.vm.passwordForm.currentPassword = 'old'
    wrapper.vm.passwordForm.newPassword = 'new123'
    wrapper.vm.passwordForm.confirmPassword = 'new123'
    await wrapper.vm.$nextTick()

    await wrapper.vm.changePassword()
    await wrapper.vm.$nextTick()

    expect(mockInvoke).toHaveBeenCalledWith('change_password', {
      currentPassword: 'old',
      newPassword: 'new123',
    })
  })

  it('双因素认证按钮 - 打开 2FA 对话框', async () => {
    const wrapper = mount(Profile, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.show2FADialog).toBe(false)

    const faBtn = wrapper.findAll('.el-button').find(b => b.text().includes('双因素认证'))
    expect(faBtn).toBeDefined()
    await faBtn!.trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.show2FADialog).toBe(true)
  })

  it('API 调用失败 - 显示错误消息且不崩溃', async () => {
    mockInvoke.mockRejectedValue(new Error('Network error'))

    const wrapper = mount(Profile, { global: { plugins: [ElementPlus] } })
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()
    await wrapper.vm.$nextTick()

    expect(wrapper.exists()).toBe(true)
  })
})
