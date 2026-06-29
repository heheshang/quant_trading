<template>
  <el-dialog v-model="dialogVisible" title="双因素认证" width="540px" @update:model-value="onVisibilityChange">
    <template v-if="!is2FAEnabled && !show2FASetup">
      <div style="text-align:center;padding:20px 0;">
        <el-icon :size="48" color="#409EFF"><Lock /></el-icon>
        <p style="margin-top:12px;font-size:14px;color:#666;">
          双因素认证为您的账户提供额外的安全保护。启用后，登录时需要输入动态验证码。
        </p>
        <el-button type="primary" @click="startSetup" style="margin-top:16px;">
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
        <p style="margin-bottom:12px;font-size:14px;color:#909399;">
          两步验证功能开发中，敬请期待
        </p>
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
          <el-icon :size="48" color="#67C23A"><CircleCheck /></el-icon>
          <p style="margin-top:12px;font-size:16px;font-weight:bold;color:#333;">双因素认证已启用</p>
          <p style="font-size:14px;color:#666;margin-top:8px;">
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
        <el-icon :size="48" color="#67C23A"><CircleCheck /></el-icon>
        <p style="margin-top:12px;font-size:16px;font-weight:bold;color:#333;">双因素认证已启用</p>
        <p style="font-size:14px;color:#666;margin-top:8px;margin-bottom:16px;">
          您的账户已受到双因素认证保护
        </p>
        <el-button type="danger" @click="showDisableConfirm = true">禁用 2FA</el-button>
      </div>
    </template>

    <template #footer>
      <span v-if="is2FAEnabled" class="dialog-footer">
        <el-button @click="dialogVisible = false">关闭</el-button>
      </span>
    </template>

    <ConfirmDialog
      v-model:visible="showDisableConfirm"
      title="确认禁用"
      message="确定要禁用双因素认证吗？这会降低账户安全性。"
      type="warning"
      confirm-text="禁用"
      @confirm="confirmDisable"
    />
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { Lock, CircleCheck } from '@element-plus/icons-vue'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const dialogVisible = computed({
  get: () => props.visible,
  set: (val: boolean) => emit('update:visible', val),
})

const is2FAEnabled = ref(false)
const show2FASetup = ref(false)
const twoFAStep = ref(1)
const verifying = ref(false)
const twoFACode = ref('')
const showDisableConfirm = ref(false)

function startSetup() {
  ElMessage.info('两步验证设置功能开发中，敬请期待')
}

function onCodeInput(value: string) {
  twoFACode.value = value.replace(/\D/g, '').slice(0, 6)
}

function verify() {
  ElMessage.info('两步验证功能开发中，敬请期待')
}

function closeDialog() {
  dialogVisible.value = false
  show2FASetup.value = false
  twoFAStep.value = 1
  twoFACode.value = ''
}

function confirmDisable() {
  ElMessage.info('两步验证功能开发中，敬请期待')
  showDisableConfirm.value = false
}

function onVisibilityChange(val: boolean) {
  if (!val) {
    show2FASetup.value = false
    twoFAStep.value = 1
    twoFACode.value = ''
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
