<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>基本配置</span>
      </div>
    </template>

    <el-form ref="formRef" :model="model" :rules="rules" label-width="150px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="系统名称" prop="name">
            <el-input v-model="model.name" placeholder="输入系统名称" />
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="系统版本" prop="version">
            <el-input v-model="model.version" placeholder="输入系统版本" readonly />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="语言" prop="language">
            <el-select v-model="model.language" placeholder="选择语言" style="width: 100%">
              <el-option label="中文" value="zh-CN" />
              <el-option label="English" value="en-US" />
            </el-select>
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="时区" prop="timezone">
            <el-select v-model="model.timezone" placeholder="选择时区" style="width: 100%">
              <el-option label="UTC" value="UTC" />
              <el-option label="UTC+8 (北京)" value="UTC+8" />
              <el-option label="UTC-5 (纽约)" value="UTC-5" />
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

defineOptions({ name: 'SettingsBasic' })

export interface SystemInfo {
  name: string
  version: string
  language: string
  timezone: string
}

const model = defineModel<SystemInfo>({ required: true })
const formRef = ref<FormInstance>()

const rules: FormRules = {
  name: [
    { required: true, message: '请输入系统名称', trigger: 'blur' },
  ],
  version: [
    { required: true, message: '系统版本不能为空', trigger: 'blur' },
  ],
  language: [
    { required: true, message: '请选择语言', trigger: 'change' },
  ],
  timezone: [
    { required: true, message: '请选择时区', trigger: 'change' },
  ],
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
