<template>
  <el-card class="risk-config-card">
    <template #header>
      <div class="card-header">
        <span>风险配置</span>
        <el-button type="primary" :loading="saving" @click="handleSave">保存配置</el-button>
      </div>
    </template>

    <el-form ref="formRef" :model="config" :rules="rules" label-width="150px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="最大持仓比例" prop="max_position_size">
            <el-slider v-model="config.max_position_size" :min="0" :max="1" :step="0.01" show-input style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="单日最大亏损比例" prop="max_daily_loss">
            <el-slider v-model="config.max_daily_loss" :min="0" :max="0.2" :step="0.001" show-input style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="最大回撤限制" prop="max_drawdown">
            <el-slider v-model="config.max_drawdown" :min="0" :max="0.3" :step="0.01" show-input style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="VaR置信水平" prop="var_confidence_level">
            <el-slider v-model="config.var_confidence_level" :min="0.9" :max="0.999" :step="0.001" show-input style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="启用事前检查" prop="enable_pre_trade_check">
            <el-switch v-model="config.enable_pre_trade_check" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="启用实时监控" prop="enable_real_time_monitor">
            <el-switch v-model="config.enable_real_time_monitor" />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { FormInstance } from 'element-plus'
import type { RiskConfig } from '@/services/types'

const props = defineProps<{
  config: RiskConfig
  saving: boolean
}>()

const emit = defineEmits<{
  save: [config: RiskConfig]
}>()

const formRef = ref<FormInstance>()

const rules = {
  max_position_size: [
    { required: true, message: '请设置最大持仓比例', trigger: 'change' },
    { type: 'number', min: 0, max: 1, message: '最大持仓比例应在0-1之间', trigger: 'change' },
  ],
  max_daily_loss: [
    { required: true, message: '请设置单日最大亏损比例', trigger: 'change' },
    { type: 'number', min: 0, max: 0.2, message: '单日最大亏损比例应在0-0.2之间', trigger: 'change' },
  ],
  max_drawdown: [
    { required: true, message: '请设置最大回撤限制', trigger: 'change' },
    { type: 'number', min: 0, max: 0.3, message: '最大回撤限制应在0-0.3之间', trigger: 'change' },
  ],
  var_confidence_level: [
    { required: true, message: '请设置VaR置信水平', trigger: 'change' },
    { type: 'number', min: 0.9, max: 0.999, message: 'VaR置信水平应在0.9-0.999之间', trigger: 'change' },
  ],
}

async function handleSave() {
  if (!formRef.value) return
  await formRef.value.validate((valid) => {
    if (!valid) return
    emit('save', { ...props.config })
  })
}
</script>

<style scoped>
.risk-config-card {
  margin-bottom: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
