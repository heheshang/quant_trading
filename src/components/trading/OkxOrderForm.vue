<template>
  <el-card class="okx-section-card">
    <template #header>
      <div class="card-header">
        <span>OKX 下单</span>
      </div>
    </template>

    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :rules="formRules"
      inline
    >
      <el-form-item label="交易对" prop="instId">
        <el-select
          v-model="formData.instId"
          placeholder="选择交易对"
          style="width: 160px"
          filterable
        >
          <el-option
            v-for="inst in instruments"
            :key="inst.instId"
            :label="inst.instId"
            :value="inst.instId"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="方向" prop="side">
        <el-select v-model="formData.side" style="width: 120px">
          <el-option label="买入" value="buy" />
          <el-option label="卖出" value="sell" />
        </el-select>
      </el-form-item>

      <el-form-item label="类型" prop="ordType">
        <el-select v-model="formData.ordType" style="width: 120px">
          <el-option label="限价" value="limit" />
          <el-option label="市价" value="market" />
        </el-select>
      </el-form-item>

      <el-form-item label="价格" prop="px">
        <el-input-number
          v-model="formData.px"
          :min="0"
          :precision="2"
          :step="0.01"
          style="width: 160px"
        />
      </el-form-item>

      <el-form-item label="数量" prop="sz">
        <el-input-number
          v-model="formData.sz"
          :min="0"
          :precision="4"
          :step="0.001"
          style="width: 160px"
        />
      </el-form-item>

      <el-form-item>
        <el-button
          type="primary"
          :loading="submitting"
          :disabled="!connected"
          @click="handleSubmit"
        >
          提交
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import type { FormInstance } from 'element-plus'

export interface OkxOrderFormData {
  instId: string
  side: string
  ordType: string
  px: number
  sz: number
}

export interface Instrument {
  instId: string
  baseCcy?: string
  quoteCcy?: string
}

withDefaults(
  defineProps<{
    instruments: Instrument[]
    connected: boolean
    submitting: boolean
  }>(),
  {
    instruments: () => [],
    connected: false,
    submitting: false,
  },
)

const emit = defineEmits<{
  submit: [data: OkxOrderFormData]
}>()

const formRef = ref<FormInstance>()

const formData = reactive<OkxOrderFormData>({
  instId: '',
  side: 'buy',
  ordType: 'limit',
  px: 0,
  sz: 0,
})

const formRules = {
  instId: [{ required: true, message: '请选择交易对', trigger: 'change' }],
  side: [{ required: true, message: '请选择方向', trigger: 'change' }],
  ordType: [{ required: true, message: '请选择类型', trigger: 'change' }],
}

async function handleSubmit() {
  if (!formRef.value) return

  await formRef.value.validate((valid: boolean) => {
    if (!valid) return
    emit('submit', { ...formData })
  })
}
</script>

<style scoped>
.okx-section-card {
  margin-bottom: 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
