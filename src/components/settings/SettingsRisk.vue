<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>风险配置</span>
      </div>
    </template>

    <el-form ref="formRef" :model="model" :rules="rules" label-width="150px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="最大持仓比例" prop="max_position_size">
            <el-slider
              v-model="model.max_position_size"
              :min="0"
              :max="1"
              :step="0.01"
              show-input
              style="width: 100%"
            />
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="单日最大亏损比例" prop="max_daily_loss">
            <el-slider
              v-model="model.max_daily_loss"
              :min="0"
              :max="0.2"
              :step="0.001"
              show-input
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="最大回撤限制" prop="max_drawdown">
            <el-slider
              v-model="model.max_drawdown"
              :min="0"
              :max="0.3"
              :step="0.01"
              show-input
              style="width: 100%"
            />
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="最大集中度" prop="max_concentration">
            <el-slider
              v-model="model.max_concentration"
              :min="0"
              :max="1"
              :step="0.01"
              show-input
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="VaR置信水平" prop="var_confidence_level">
            <el-slider
              v-model="model.var_confidence_level"
              :min="0.9"
              :max="0.999"
              :step="0.001"
              show-input
              style="width: 100%"
            />
          </el-form-item>
        </el-col>

        <el-col :span="12">
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="启用事前检查" prop="enable_pre_trade_check">
            <el-switch v-model="model.enable_pre_trade_check" />
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="启用实时监控" prop="enable_real_time_monitor">
            <el-switch v-model="model.enable_real_time_monitor" />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

defineOptions({ name: 'SettingsRisk' })

export interface RiskConfig {
  max_position_size: number
  max_daily_loss: number
  max_drawdown: number
  max_concentration: number
  var_confidence_level: number
  enable_pre_trade_check: boolean
  enable_real_time_monitor: boolean
}

const model = defineModel<RiskConfig>({ required: true })
const formRef = ref<FormInstance>()

const rules: FormRules = {
  max_position_size: [
    { required: true, message: '请输入最大持仓比例', trigger: 'blur' },
    { type: 'number', min: 0, max: 1, message: '持仓比例范围 0-1', trigger: 'blur' },
  ],
  max_daily_loss: [
    { required: true, message: '请输入单日最大亏损比例', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.2, message: '亏损比例范围 0-0.2', trigger: 'blur' },
  ],
  max_drawdown: [
    { required: true, message: '请输入最大回撤限制', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.3, message: '回撤范围 0-0.3', trigger: 'blur' },
  ],
  max_concentration: [
    { required: true, message: '请输入最大集中度', trigger: 'blur' },
    { type: 'number', min: 0, max: 1, message: '集中度范围 0-1', trigger: 'blur' },
  ],
  var_confidence_level: [
    { required: true, message: '请输入VaR置信水平', trigger: 'blur' },
    { type: 'number', min: 0.9, max: 0.999, message: '置信水平范围 0.9-0.999', trigger: 'blur' },
  ],
  enable_pre_trade_check: [],
  enable_real_time_monitor: [],
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
