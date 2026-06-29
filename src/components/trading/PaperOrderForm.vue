<template>
  <el-card class="order-form-card">
    <template #header>
      <div class="card-header">
        <span>创建订单</span>
      </div>
    </template>

    <el-form
      ref="formRef"
      :model="formData"
      label-width="100px"
      :rules="formRules"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="策略" prop="strategy_id">
            <el-select
              v-model="formData.strategy_id"
              placeholder="选择策略"
              style="width: 100%"
            >
              <el-option
                v-for="s in strategies"
                :key="s.strategy_id"
                :label="s.strategy_name"
                :value="s.strategy_id"
              />
            </el-select>
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="标的代码" prop="symbol">
            <el-input
              v-model="formData.symbol"
              placeholder="输入标的代码，如 600519.SH"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="买卖方向" prop="side">
            <el-select
              v-model="formData.side"
              placeholder="选择买卖方向"
              style="width: 100%"
            >
              <el-option label="买入" value="Buy" />
              <el-option label="卖出" value="Sell" />
            </el-select>
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="订单类型" prop="order_type">
            <el-select
              v-model="formData.order_type"
              placeholder="选择订单类型"
              style="width: 100%"
            >
              <el-option label="限价单" value="Limit" />
              <el-option label="市价单" value="Market" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="价格" prop="price">
            <el-input-number
              v-model="formData.price"
              :min="0"
              :precision="2"
              :step="0.01"
              style="width: 100%"
              :disabled="formData.order_type === 'Market'"
            />
          </el-form-item>
        </el-col>

        <el-col :span="12">
          <el-form-item label="数量" prop="quantity">
            <el-input-number
              v-model="formData.quantity"
              :min="0"
              :precision="2"
              :step="100"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-form-item>
        <el-button
          type="primary"
          :loading="submitting"
          @click="handleSubmit"
        >
          提交订单
        </el-button>
        <el-button @click="handleReset">
          重置
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import type { FormInstance } from 'element-plus'

interface Strategy {
  strategy_id: string | number
  strategy_name: string
}

export interface OrderFormData {
  strategy_id: string
  symbol: string
  side: string
  order_type: string
  price: number
  quantity: number
}

withDefaults(
  defineProps<{
    strategies: Strategy[]
    submitting: boolean
  }>(),
  {
    strategies: () => [],
    submitting: false,
  },
)

const emit = defineEmits<{
  submit: [data: OrderFormData]
  reset: []
}>()

const formRef = ref<FormInstance>()

function createDefaultFormData(): OrderFormData {
  return {
    strategy_id: '',
    symbol: '600519.SH',
    side: 'Buy',
    order_type: 'Limit',
    price: 1685.00,
    quantity: 100,
  }
}

const formData = reactive<OrderFormData>(createDefaultFormData())

const formRules = {
  strategy_id: [{ required: true, message: '请选择策略', trigger: 'change' }],
  symbol: [{ required: true, message: '请输入标的代码', trigger: 'blur' }],
  side: [{ required: true, message: '请选择买卖方向', trigger: 'change' }],
  order_type: [{ required: true, message: '请选择订单类型', trigger: 'change' }],
  price: [{ required: true, message: '请输入价格', trigger: 'blur' }],
}

async function handleSubmit() {
  if (!formRef.value) return

  await formRef.value.validate((valid: boolean) => {
    if (!valid) return
    emit('submit', { ...formData })
  })
}

function handleReset() {
  Object.assign(formData, createDefaultFormData())
  formRef.value?.resetFields()
  emit('reset')
}
</script>

<style scoped>
.order-form-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
