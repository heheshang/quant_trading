<template>
  <el-dialog v-model="dialogVisible" title="修改密码" width="500px" @update:model-value="onVisibilityChange">
    <el-form :model="passwordForm" :rules="passwordRules" ref="passwordFormRef" label-width="100px">
      <el-form-item label="当前密码" prop="currentPassword">
        <el-input v-model="passwordForm.currentPassword" type="password" show-password />
      </el-form-item>
      <el-form-item label="新密码" prop="newPassword">
        <el-input v-model="passwordForm.newPassword" type="password" show-password />
      </el-form-item>
      <el-form-item label="确认密码" prop="confirmPassword">
        <el-input v-model="passwordForm.confirmPassword" type="password" show-password />
      </el-form-item>
    </el-form>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleChangePassword" :loading="changingPassword">确定</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage, type FormInstance } from 'element-plus'
import { changePassword as apiChangePassword } from '@/services/auth'
import { useAuthStore } from '@/stores/auth'
import { useRouter } from 'vue-router'

const props = defineProps<{
  visible: boolean
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
  passwordChanged: []
}>()

const router = useRouter()
const authStore = useAuthStore()

const dialogVisible = computed({
  get: () => props.visible,
  set: (val: boolean) => emit('update:visible', val),
})

const passwordFormRef = ref<FormInstance>()
const changingPassword = ref(false)

const passwordForm = ref({
  currentPassword: '',
  newPassword: '',
  confirmPassword: '',
})

const passwordRules = {
  currentPassword: [
    { required: true, message: '请输入当前密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6位', trigger: 'blur' },
  ],
  newPassword: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6位', trigger: 'blur' },
  ],
  confirmPassword: [
    { required: true, message: '请确认新密码', trigger: 'blur' },
    {
      validator: (_rule: unknown, value: string, callback: (error?: Error) => void) => {
        if (value !== passwordForm.value.newPassword) {
          callback(new Error('两次输入的密码不一致'))
        } else {
          callback()
        }
      },
      trigger: 'blur',
    },
  ],
}

function onVisibilityChange(val: boolean) {
  if (!val) {
    passwordForm.value = { currentPassword: '', newPassword: '', confirmPassword: '' }
  }
}

async function handleChangePassword() {
  if (!passwordFormRef.value) return

  await passwordFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    changingPassword.value = true
    try {
      const username = localStorage.getItem('username') || undefined
      const result = await apiChangePassword(
        passwordForm.value.currentPassword,
        passwordForm.value.newPassword,
        username,
      )

      if (result) {
        ElMessage.success('密码修改成功，即将跳转至登录页面')
        dialogVisible.value = false
        passwordForm.value = { currentPassword: '', newPassword: '', confirmPassword: '' }
        emit('passwordChanged')
        authStore.clearSession()
        setTimeout(() => {
          router.push('/login')
        }, 2000)
      } else {
        ElMessage.error('密码修改失败')
      }
    } catch (error) {
      console.error('Failed to change password:', error)
      ElMessage.error('密码修改失败: ' + (error as Error).message)
    } finally {
      changingPassword.value = false
    }
  })
}

defineExpose({
  passwordFormRef,
  passwordForm,
  changingPassword,
  handleChangePassword,
})
</script>

<style scoped>
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
