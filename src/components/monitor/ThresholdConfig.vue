<template>
  <el-row :gutter="20">
    <el-col :span="24">
      <el-card>
        <template #header>
          <div class="card-header">
            <span>阈值配置</span>
            <el-button type="primary" size="small" @click="handleSave">保存配置</el-button>
          </div>
        </template>
        <el-form label-width="160px">
          <el-row :gutter="20">
            <el-col :xs="24" :span="12">
              <el-form-item label="最大回撤阈值">
                <el-input-number v-model="local.maxDrawdown" :min="0" :max="100" :precision="1" :step="0.5">
                  <template #suffix>%</template>
                </el-input-number>
              </el-form-item>
              <el-form-item label="日亏损阈值">
                <el-input-number v-model="local.dailyLoss" :min="0" :max="100" :precision="1" :step="0.5">
                  <template #suffix>%</template>
                </el-input-number>
              </el-form-item>
              <el-form-item label="持仓集中度">
                <el-input-number v-model="local.concentration" :min="0" :max="100" :precision="1" :step="1">
                  <template #suffix>%</template>
                </el-input-number>
              </el-form-item>
            </el-col>
            <el-col :xs="24" :span="12">
              <el-form-item label="杠杆率上限">
                <el-input-number v-model="local.leverage" :min="1" :max="10" :precision="1" :step="0.5">
                  <template #suffix>x</template>
                </el-input-number>
              </el-form-item>
              <el-form-item label="订单延迟告警">
                <el-input-number v-model="local.orderLatency" :min="0" :max="10000" :step="10">
                  <template #suffix>ms</template>
                </el-input-number>
              </el-form-item>
              <el-form-item label="VaR 预警阈值">
                <el-input-number v-model="local.varWarning" :min="0" :max="100" :precision="1" :step="0.5">
                  <template #suffix>%</template>
                </el-input-number>
              </el-form-item>
            </el-col>
          </el-row>
        </el-form>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import type { ThresholdConfig } from './types'

const props = defineProps<{
  config: ThresholdConfig
}>()

const emit = defineEmits<{
  save: [value: ThresholdConfig]
}>()

const local = reactive<ThresholdConfig>({ ...props.config })

watch(() => props.config, (val) => {
  Object.assign(local, val)
}, { deep: true })

function handleSave() {
  emit('save', { ...local })
}
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
