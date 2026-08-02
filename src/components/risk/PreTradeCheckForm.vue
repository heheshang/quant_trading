<template>
  <el-card class="pre-trade-check-card">
    <template #header>
      <div class="card-header">
        <span>事前风控测试</span>
      </div>
    </template>

    <el-form ref="formRef" :model="testOrder" :rules="rules" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="标的代码" prop="symbol">
            <el-input v-model="testOrder.symbol" placeholder="输入标的代码" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="买卖方向" prop="side">
            <el-select v-model="testOrder.side" placeholder="选择方向" style="width: 100%">
              <el-option label="买入" value="Buy" />
              <el-option label="卖出" value="Sell" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="价格" prop="price">
            <el-input-number v-model="testOrder.price" :min="0" :precision="2" :step="0.01" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="数量" prop="quantity">
            <el-input-number v-model="testOrder.quantity" :min="0" :precision="2" :step="100" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item>
        <el-button type="primary" :loading="checking" @click="runCheck">风控检查</el-button>
        <el-button @click="resetForm">重置</el-button>
      </el-form-item>
    </el-form>

    <el-alert
      v-if="checkResult !== null"
      :type="checkResult ? 'success' : 'error'"
      :title="checkResult ? '风控检查通过' : '风控检查未通过'"
      :closable="false"
      show-icon
    />
  </el-card>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { getAccountInfo, getPositions } from '@/services/account'
import { preTradeCheck } from '@/services/risk'

const formRef = ref<FormInstance>()
const checkResult = ref<boolean | null>(null)
const checking = ref(false)

const testOrder = reactive({
  order_id: 0,
  strategy_id: 'test_strategy',
  symbol: '600519.SH',
  order_type: 'Limit' as const,
  side: 'Buy' as const,
  price: 1685.00,
  quantity: 100,
  filled_quantity: 0,
  status: 'Pending' as const,
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  commission: 0,
  slippage: 0,
})

const rules = {
  symbol: [{ required: true, message: '请输入标的代码', trigger: 'blur' }],
  side: [{ required: true, message: '请选择买卖方向', trigger: 'change' }],
  price: [
    { required: true, message: '请输入价格', trigger: 'blur' },
    { type: 'number', min: 0, message: '价格不能为负数', trigger: 'blur' },
  ],
  quantity: [
    { required: true, message: '请输入数量', trigger: 'blur' },
    { type: 'number', min: 0, message: '数量不能为负数', trigger: 'blur' },
  ],
}

async function runCheck() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    checking.value = true
    try {
      testOrder.order_id = Date.now()
      const [account, positions] = await Promise.all([
        getAccountInfo(),
        getPositions(),
      ])
      const result = await preTradeCheck(testOrder, account, positions)
      checkResult.value = result
      ElMessage.success(result ? '风控检查通过' : '风控检查未通过')
    } catch (error) {
      console.error('Failed to run pre-trade check:', error)
      ElMessage.error('风控检查失败: ' + (error as Error).message)
    } finally {
      checking.value = false
    }
  })
}

function resetForm() {
  testOrder.order_id = 0
  testOrder.symbol = '600519.SH'
  testOrder.side = 'Buy'
  testOrder.price = 1685.00
  testOrder.quantity = 100
  checkResult.value = null
}

defineExpose({
  formRef,
  testOrder,
  checkResult,
  checking,
  runCheck,
  resetForm,
})
</script>

<style scoped>
.pre-trade-check-card {
  margin-bottom: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
