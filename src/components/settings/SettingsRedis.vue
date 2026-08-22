<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>Redis配置</span>
      </div>
    </template>

    <el-form ref="formRef" :model="model" :rules="rules" label-width="120px">
      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="主机地址" prop="host">
            <el-input v-model="model.host" placeholder="输入Redis主机地址" />
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
          <el-form-item label="密码" prop="password">
            <el-input
              v-model="model.password"
              type="password"
              placeholder="输入Redis密码（可选）"
              show-password
            />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="数据库" prop="db">
            <el-input-number
              v-model="model.db"
              :min="0"
              :max="15"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="连接池大小" prop="pool_size">
            <el-input-number
              v-model="model.pool_size"
              :min="1"
              :max="100"
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

defineOptions({ name: 'SettingsRedis' })

export interface RedisConfig {
  host: string
  port: number
  password: string | null
  db: number
  pool_size: number
}

const model = defineModel<RedisConfig>({ required: true })
const formRef = ref<FormInstance>()

const rules: FormRules = {
  host: [
    { required: true, message: '请输入Redis主机地址', trigger: 'blur' },
  ],
  port: [
    { required: true, message: '请输入Redis端口', trigger: 'blur' },
    { type: 'number', min: 1, max: 65535, message: '端口范围 1-65535', trigger: 'blur' },
  ],
  password: [],
  db: [],
  pool_size: [
    { required: true, message: '请输入连接池大小', trigger: 'blur' },
    { type: 'number', min: 1, max: 100, message: '连接池大小范围 1-100', trigger: 'blur' },
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
