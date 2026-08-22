<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>数据库配置</span>
      </div>
    </template>

    <el-form ref="formRef" :model="model" :rules="rules" label-width="120px">
      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="主机地址" prop="host">
            <el-input v-model="model.host" placeholder="输入数据库主机地址" />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="端口" prop="port">
            <el-input-number
              v-model="model.port"
              :min="1"
              :max="65535"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="用户名" prop="username">
            <el-input v-model="model.username" placeholder="输入数据库用户名" />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="密码" prop="password">
            <el-input
              v-model="model.password"
              type="password"
              placeholder="输入数据库密码"
              show-password
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="数据库名" prop="database">
            <el-input v-model="model.database" placeholder="输入数据库名" />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="最大连接数" prop="max_connections">
            <el-input-number
              v-model="model.max_connections"
              :min="1"
              :max="1000"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

defineOptions({ name: 'SettingsDatabase' })

export interface DatabaseConfig {
  host: string
  port: number
  username: string
  password: string | null
  database: string
  max_connections: number
  connect_timeout_seconds?: number
}

const model = defineModel<DatabaseConfig>({ required: true })
const formRef = ref<FormInstance>()

const rules: FormRules = {
  host: [
    { required: true, message: '请输入数据库主机地址', trigger: 'blur' },
  ],
  port: [
    { required: true, message: '请输入数据库端口', trigger: 'blur' },
    { type: 'number', min: 1, max: 65535, message: '端口范围 1-65535', trigger: 'blur' },
  ],
  username: [
    { required: true, message: '请输入数据库用户名', trigger: 'blur' },
  ],
  password: [],
  database: [
    { required: true, message: '请输入数据库名', trigger: 'blur' },
  ],
  max_connections: [
    { required: true, message: '请输入最大连接数', trigger: 'blur' },
    { type: 'number', min: 1, max: 1000, message: '连接数范围 1-1000', trigger: 'blur' },
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
