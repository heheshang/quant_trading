<template>
  <el-dialog v-model="dialogVisible" title="双因素认证" width="540px" @update:model-value="onVisibilityChange">
    <template v-if="!is2FAEnabled && !show2FASetup">
      <div style="text-align:center;padding:20px 0;">
        <el-icon :size="48" color="var(--color-primary)"><Lock /></el-icon>
        <p style="margin-top:12px;font-size:14px;color:var(--color-text-regular);">
          双因素认证为您的账户提供额外的安全保护。启用后，登录时需要输入动态验证码。
        </p>
        <el-button type="primary" @click="startSetup" :loading="verifying" style="margin-top:16px;">
          立即启用
        </el-button>
      </div>
    </template>

    <template v-else-if="show2FASetup">
      <el-steps :active="twoFAStep" simple style="margin-bottom:20px">
        <el-step title="绑定密钥" :status="twoFAStep > 1 ? 'finish' : 'process'" />
        <el-step title="验证代码" :status="twoFAStep > 2 ? 'finish' : twoFAStep === 2 ? 'process' : 'wait'" />
        <el-step title="完成" :status="twoFAStep > 3 ? 'finish' : 'wait'" />
      </el-steps>

      <div v-if="twoFAStep === 1">
        <p style="margin-bottom:12px;font-size:14px;color:var(--color-text-secondary);">
          请使用验证器应用（如 Google Authenticator）扫描下方密钥，或手动输入：
        </p>
        <div style="text-align:center;padding:12px 0;">
          <div style="font-family:monospace;font-size:16px;letter-spacing:2px;word-break:break-all;background:var(--color-fill-light);padding:12px;border-radius:6px;">
            {{ secret }}
          </div>
          <p style="margin-top:10px;font-size:12px;color:var(--color-text-secondary);word-break:break-all;">
            {{ otpauthUri }}
          </p>
        </div>
        <div style="text-align:right;margin-top:12px;">
          <el-button @click="closeDialog">取消</el-button>
          <el-button type="primary" @click="twoFAStep = 2">下一步，输入验证码</el-button>
        </div>
      </div>

      <div v-if="twoFAStep === 2">
        <p style="margin-bottom:12px;font-size:14px;">
          请输入您的验证器应用当前显示的 6 位动态验证码：
        </p>
        <div style="text-align:center;padding:12px 0;">
          <el-input
            v-model="twoFACode"
            :maxlength="6"
            placeholder="000000"
            size="large"
            style="width:200px;font-size:24px;text-align:center;"
            @input="onCodeInput"
          />
        </div>
        <div style="text-align:right;margin-top:12px;">
          <el-button @click="twoFAStep = 1">返回</el-button>
          <el-button type="primary" @click="verify" :loading="verifying">验证并启用</el-button>
        </div>
      </div>

      <div v-if="twoFAStep === 3">
        <div style="text-align:center;padding:20px 0;">
          <el-icon :size="48" color="var(--color-success)"><CircleCheck /></el-icon>
          <p style="margin-top:12px;font-size:16px;font-weight:bold;color:var(--color-text-primary);">双因素认证已启用</p>
          <p style="font-size:14px;color:var(--color-text-regular);margin-top:8px;">
            今后登录时，您需要输入手机验证码以及动态验证码
          </p>
        </div>
        <div style="text-align:right;">
          <el-button type="primary" @click="closeDialog">完成</el-button>
        </div>
      </div>
    </template>

    <template v-else>
      <div style="text-align:center;padding:20px 0;">
        <template v-if="!showDisableForm">
          <el-icon :size="48" color="var(--color-success)"><CircleCheck /></el-icon>
          <p style="margin-top:12px;font-size:16px;font-weight:bold;color:var(--color-text-primary);">双因素认证已启用</p>
          <p style="font-size:14px;color:var(--color-text-regular);margin-top:8px;margin-bottom:16px;">
            您的账户已受到双因素认证保护
          </p>
          <el-button type="danger" @click="showDisableForm = true">禁用 2FA</el-button>
        </template>
        <template v-else>
          <p style="margin-bottom:12px;font-size:14px;">
            请输入您的验证器应用当前显示的 6 位动态验证码以关闭双因素认证：
          </p>
          <div style="text-align:center;padding:12px 0;">
            <el-input
              v-model="disableCode"
              :maxlength="6"
              placeholder="000000"
              size="large"
              style="width:200px;font-size:24px;text-align:center;"
              @input="onDisableCodeInput"
            />
          </div>
          <div style="text-align:right;margin-top:12px;">
            <el-button @click="cancelDisable">取消</el-button>
            <el-button type="danger" @click="confirmDisable" :loading="verifying">确认禁用</el-button>
          </div>
        </template>
      </div>
    </template>

    <template #footer>
      <span v-if="is2FAEnabled" class="dialog-footer">
        <el-button @click="dialogVisible = false">关闭</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { Lock, CircleCheck } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { enable2FA, verify2FACode, disable2FA } from '@/services/twoFA'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const authStore = useAuthStore()
// The auth session does not carry a numeric subject yet (stores/auth.ts sets
// currentUser.id to 0 until a real JWT subject is decoded); callers pass 0
// for now, matching the existing api-key convention.
const userId = computed(() => authStore.currentUser?.id ?? 0)

const dialogVisible = computed({
  get: () => props.visible,
  set: (val: boolean) => emit('update:visible', val),
})

const is2FAEnabled = ref(false)
const show2FASetup = ref(false)
const twoFAStep = ref(1)
const verifying = ref(false)
const twoFACode = ref('')
const secret = ref('')
const otpauthUri = ref('')
const showDisableForm = ref(false)
const disableCode = ref('')

async function startSetup() {
  verifying.value = true
  try {
    const result = await enable2FA(userId.value)
    secret.value = result.secret
    otpauthUri.value = result.otpauth_uri
    show2FASetup.value = true
    twoFAStep.value = 1
  } catch (error) {
    console.error('Failed to enable 2FA:', error)
    ElMessage.error('开启双因素认证失败')
  } finally {
    verifying.value = false
  }
}

function onCodeInput(value: string) {
  twoFACode.value = value.replace(/\D/g, '').slice(0, 6)
}

function onDisableCodeInput(value: string) {
  disableCode.value = value.replace(/\D/g, '').slice(0, 6)
}

async function verify() {
  if (twoFACode.value.length !== 6) {
    ElMessage.warning('请输入 6 位动态验证码')
    return
  }
  verifying.value = true
  try {
    const ok = await verify2FACode(userId.value, twoFACode.value)
    if (ok) {
      is2FAEnabled.value = true
      twoFAStep.value = 3
    } else {
      ElMessage.error('验证码错误，请重试')
    }
  } catch (error) {
    console.error('Failed to verify 2FA code:', error)
    ElMessage.error('验证失败')
  } finally {
    verifying.value = false
  }
}

function closeDialog() {
  dialogVisible.value = false
  show2FASetup.value = false
  twoFAStep.value = 1
  twoFACode.value = ''
  secret.value = ''
  otpauthUri.value = ''
}

function cancelDisable() {
  showDisableForm.value = false
  disableCode.value = ''
}

async function confirmDisable() {
  if (disableCode.value.length !== 6) {
    ElMessage.warning('请输入 6 位动态验证码')
    return
  }
  verifying.value = true
  try {
    // `disable_2fa` validates the code server-side and refuses when invalid.
    const ok = await disable2FA(userId.value, disableCode.value)
    if (ok) {
      is2FAEnabled.value = false
      showDisableForm.value = false
      disableCode.value = ''
      ElMessage.success('双因素认证已禁用')
    } else {
      ElMessage.error('验证码错误，禁用已拒绝')
    }
  } catch (error) {
    console.error('Failed to disable 2FA:', error)
    ElMessage.error('禁用失败')
  } finally {
    verifying.value = false
  }
}

function onVisibilityChange(val: boolean) {
  if (!val) {
    show2FASetup.value = false
    twoFAStep.value = 1
    twoFACode.value = ''
    secret.value = ''
    otpauthUri.value = ''
    showDisableForm.value = false
    disableCode.value = ''
  }
}
</script>

<style scoped>
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
