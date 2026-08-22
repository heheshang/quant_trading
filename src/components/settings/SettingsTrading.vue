<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>交易配置</span>
      </div>
    </template>

    <el-form ref="formRef" :model="model" :rules="rules" label-width="150px">
      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="模拟交易" prop="enable_paper_trading">
            <el-switch v-model="model.enable_paper_trading" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="每秒最大订单数" prop="max_orders_per_second">
            <el-input-number
              v-model="model.max_orders_per_second"
              :min="1"
              :max="1000"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="默认手续费率" prop="default_commission_rate">
            <el-input-number
              v-model="model.default_commission_rate"
              :min="0"
              :max="0.1"
              :step="0.0001"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
          <el-form-item label="默认滑点" prop="default_slippage">
            <el-input-number
              v-model="model.default_slippage"
              :min="0"
              :max="0.1"
              :step="0.0001"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>

        <el-col :xs="24" :span="12">
          <el-form-item label="订单超时时间(秒)" prop="order_timeout_seconds">
            <el-input-number
              v-model="model.order_timeout_seconds"
              :min="1"
              :max="3600"
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

defineOptions({ name: 'SettingsTrading' })

export interface TradingConfig {
  enable_paper_trading: boolean
  max_orders_per_second: number
  default_commission_rate: number
  default_slippage: number
  order_timeout_seconds: number
}

const model = defineModel<TradingConfig>({ required: true })
const formRef = ref<FormInstance>()

const rules: FormRules = {
  enable_paper_trading: [],
  max_orders_per_second: [
    { required: true, message: '请输入每秒最大订单数', trigger: 'blur' },
    { type: 'number', min: 1, max: 1000, message: '订单数范围 1-1000', trigger: 'blur' },
  ],
  default_commission_rate: [
    { required: true, message: '请输入默认手续费率', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '手续费率范围 0-0.1', trigger: 'blur' },
  ],
  default_slippage: [
    { required: true, message: '请输入默认滑点', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '滑点范围 0-0.1', trigger: 'blur' },
  ],
  order_timeout_seconds: [
    { required: true, message: '请输入订单超时时间', trigger: 'blur' },
    { type: 'number', min: 1, max: 3600, message: '超时范围 1-3600秒', trigger: 'blur' },
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
