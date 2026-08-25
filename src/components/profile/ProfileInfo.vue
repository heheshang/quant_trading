<template>
  <el-card class="account-info-card">
    <template #header>
      <div class="card-header">
        <span>账户信息</span>
        <el-button
          v-if="!isEditing"
          type="primary"
          @click="$emit('startEdit')"
          :loading="loading"
        >
          编辑信息
        </el-button>
        <div v-else>
          <el-button @click="$emit('cancelEdit')">取消</el-button>
          <el-button type="primary" @click="handleSave" :loading="saving">保存</el-button>
        </div>
      </div>
    </template>

    <el-form
      :model="localForm"
      :rules="profileRules"
      ref="profileFormRef"
      label-width="120px"
      :disabled="!isEditing"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="账户ID" prop="account_id">
            <el-input v-model="localForm.account_id" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="用户名" prop="username">
            <el-input v-model="localForm.username" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="邮箱" prop="email">
            <el-input v-model="localForm.email" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="手机号" prop="phone">
            <el-input v-model="localForm.phone" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="姓名" prop="full_name">
            <el-input v-model="localForm.full_name" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="公司" prop="company">
            <el-input v-model="localForm.company" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="24">
          <el-form-item label="地址" prop="address">
            <el-input v-model="localForm.address" type="textarea" />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import type { FormInstance } from 'element-plus'

export interface ProfileFormData {
  account_id: string
  username: string
  email: string
  phone: string
  full_name: string
  company: string
  address: string
}

const props = defineProps<{
  initialForm: ProfileFormData
  isEditing: boolean
  saving: boolean
  loading: boolean
}>()

const emit = defineEmits<{
  startEdit: []
  cancelEdit: []
  save: [formData: ProfileFormData]
}>()

const profileFormRef = ref<FormInstance>()

const localForm = reactive<ProfileFormData>({ ...props.initialForm })

watch(() => props.initialForm, (v) => {
  Object.assign(localForm, v)
}, { deep: true })

const profileRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '用户名长度应在3-20个字符之间', trigger: 'blur' },
  ],
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入正确的邮箱地址', trigger: 'blur' },
  ],
  phone: [
    { required: true, message: '请输入手机号', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' },
  ],
}

async function handleSave() {
  if (!profileFormRef.value) return
  try {
    await profileFormRef.value.validate()
    emit('save', { ...localForm })
  } catch {
    // validation failed — form handles error display
  }
}
</script>

<style scoped>

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
