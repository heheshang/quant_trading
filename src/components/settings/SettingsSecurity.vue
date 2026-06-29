<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>安全配置</span>
      </div>
    </template>

    <el-form ref="formRef" :model="model" :rules="rules" label-width="150px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="启用加密" prop="enable_encryption">
            <el-switch v-model="model.enable_encryption" />
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="启用双因素认证" prop="enable_2fa">
            <el-switch v-model="model.enable_2fa" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="JWT密钥" prop="jwt_secret">
            <el-input
              v-model="model.jwt_secret"
              type="password"
              placeholder="输入JWT密钥"
              show-password
            />
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="Token过期时间(小时)" prop="token_expiry_hours">
            <el-input-number
              v-model="model.token_expiry_hours"
              :min="1"
              :max="720"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="24">
          <el-form-item label="允许的IP地址" prop="allowed_ips">
            <el-select
              v-model="model.allowed_ips"
              multiple
              filterable
              allow-create
              default-first-option
              placeholder="输入允许的IP地址"
              style="width: 100%"
            >
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

defineOptions({ name: 'SettingsSecurity' })

export interface SecurityConfig {
  enable_encryption: boolean
  enable_2fa: boolean
  jwt_secret: string
  token_expiry_hours: number
  allowed_ips: string[]
}

const model = defineModel<SecurityConfig>({ required: true })
const formRef = ref<FormInstance>()

const rules: FormRules = {
  enable_encryption: [],
  enable_2fa: [],
  jwt_secret: [],
  token_expiry_hours: [
    { required: true, message: '请输入Token过期时间', trigger: 'blur' },
    { type: 'number', min: 1, max: 720, message: '过期时间范围 1-720小时', trigger: 'blur' },
  ],
  allowed_ips: [],
}

defineExpose({ formRef })
</script>

<style scoped>
.settings-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
