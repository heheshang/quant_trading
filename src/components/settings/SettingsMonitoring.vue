<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>监控配置</span>
      </div>
    </template>

    <el-form ref="formRef" :model="model" :rules="rules" label-width="150px">
      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="启用Prometheus" prop="enable_prometheus">
            <el-switch v-model="model.enable_prometheus" />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="Prometheus端口" prop="prometheus_port">
            <el-input-number
              v-model="model.prometheus_port"
              :min="1"
              :max="65535"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="日志级别" prop="log_level">
            <el-select v-model="model.log_level" placeholder="选择日志级别" style="width: 100%">
              <el-option label="Debug" value="debug" />
              <el-option label="Info" value="info" />
              <el-option label="Warning" value="warning" />
              <el-option label="Error" value="error" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="告警邮箱" prop="alert_email">
            <el-input
              v-model="model.alert_email"
              placeholder="输入告警邮箱地址（可选）"
            />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="告警Webhook" prop="alert_webhook">
            <el-input
              v-model="model.alert_webhook"
              placeholder="输入告警Webhook URL（可选）"
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

defineOptions({ name: 'SettingsMonitoring' })

export interface MonitoringConfig {
  enable_prometheus: boolean
  prometheus_port: number
  log_level: string
  alert_email: string | null
  alert_webhook: string | null
}

const model = defineModel<MonitoringConfig>({ required: true })
const formRef = ref<FormInstance>()

const rules: FormRules = {
  enable_prometheus: [],
  prometheus_port: [
    { required: true, message: '请输入Prometheus端口', trigger: 'blur' },
    { type: 'number', min: 1, max: 65535, message: '端口范围 1-65535', trigger: 'blur' },
  ],
  log_level: [
    { required: true, message: '请选择日志级别', trigger: 'change' },
  ],
  alert_email: [],
  alert_webhook: [],
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
