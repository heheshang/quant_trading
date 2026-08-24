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
        <el-col :xs="24" :span="12">
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

        <el-col :xs="24" :span="12">
          <el-form-item label="标的代码" prop="symbol">
            <el-select
              v-model="formData.symbol"
              filterable
              placeholder="选择标的代码"
              style="width: 100%"
            >
              <el-option
                v-for="sym in symbols"
                :key="sym"
                :label="sym"
                :value="sym"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :xs="24" :span="12">
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

        <el-col :xs="24" :span="12">
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
        <el-col :xs="24" :span="12">
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

        <el-col :xs="24" :span="12">
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
import { ref, reactive, watch, onMounted } from 'vue'
import type { FormInstance } from 'element-plus'
import type { OrderSide, OrderType } from '@/services/types'
import { getMarketData, getSymbols } from '@/services/market'

interface Strategy {
  strategy_id: string | number
  strategy_name: string
}

export interface OrderFormData {
  strategy_id: string
  symbol: string
  side: OrderSide
  order_type: OrderType
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
    symbol: 'BTC-USDT',
    side: 'Buy',
    order_type: 'Limit',
    price: 50000,
    quantity: 0.01,
  }
}

const formData = reactive<OrderFormData>(createDefaultFormData())
/** 标的代码下拉数据源（来自数据库 market_data）。 */
const symbols = ref<string[]>([])
/** 请求序号：快速切换标的时忽略过期响应，避免旧价覆盖当前标的。 */
let priceReqSeq = 0
/** 用实时/最新行情价自动填充限价单价格（失败则保留当前值）。 */
async function refreshPrice() {
  const sym = formData.symbol?.trim()
  if (!sym) return
  const seq = ++priceReqSeq
  try {
    const md = await getMarketData(sym)
    if (seq !== priceReqSeq || md.symbol !== sym) return
    if (md && md.close > 0) {
      formData.price = md.close
    }
  } catch {
    // 行情失败时保留用户已填/默认价格
  }
}

watch(
  () => formData.symbol,
  () => {
    refreshPrice()
  },
)

onMounted(async () => {
  refreshPrice()
  try {
    symbols.value = await getSymbols()
  } catch {
    symbols.value = []
  }
})

const formRules = {
  strategy_id: [{ required: true, message: '请选择策略', trigger: 'change' }],
  symbol: [{ required: true, message: '请选择标的代码', trigger: 'change' }],
  side: [{ required: true, message: '请选择买卖方向', trigger: 'change' }],
  order_type: [{ required: true, message: '请选择订单类型', trigger: 'change' }],
  price: [
    {
      validator: (_r: unknown, value: number, cb: (e?: Error) => void) => {
        if (formData.order_type === 'Limit' && (!value || value <= 0)) {
          cb(new Error('限价单价格需 > 0'))
        } else if (value != null && value <= 0) {
          cb(new Error('价格需 > 0'))
        } else {
          cb()
        }
      },
      trigger: 'blur',
    },
  ],
  quantity: [
    { required: true, message: '请输入数量', trigger: 'blur' },
    {
      validator: (_r: unknown, value: number, cb: (e?: Error) => void) => {
        if (!value || value <= 0) cb(new Error('数量需 > 0'))
        else cb()
      },
      trigger: 'blur',
    },
  ],
}

async function handleSubmit() {
  if (!formRef.value) return

  await formRef.value.validate((valid: boolean) => {
    if (!valid) return
    emit('submit', { ...formData })
  })
}

function handleReset() {
  resetFormData()
  formRef.value?.resetFields()
  emit('reset')
}

function resetFormData() {
  Object.assign(formData, createDefaultFormData())
}

defineExpose({
  formRef,
  formData,
  handleSubmit,
  handleReset,
  resetFormData,
})
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
