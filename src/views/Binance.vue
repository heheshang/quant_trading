<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { getBinanceBalance, checkBinanceStatus } from '@/services/binance'
import { placeBinanceOrder } from '@/services/binanceOrder'
import type { BinanceBalance, BinanceStatus } from '@/services/types'

const balances = ref<BinanceBalance[]>([])
const status = ref<BinanceStatus | null>(null)
const loading = ref(false)
const submitting = ref(false)

const form = ref({
  symbol: 'BTCUSDT',
  side: 'Buy' as 'Buy' | 'Sell',
  order_type: 'Limit' as 'Market' | 'Limit',
  price: 0,
  quantity: 0,
})

async function loadBalance() {
  loading.value = true
  try {
    balances.value = await getBinanceBalance()
    status.value = await checkBinanceStatus()
  } catch (e) {
    ElMessage.error(`获取币安余额失败: ${e}`)
  } finally {
    loading.value = false
  }
}

async function submitOrder() {
  submitting.value = true
  try {
    const order = await placeBinanceOrder({
      symbol: form.value.symbol,
      side: form.value.side,
      order_type: form.value.order_type,
      price: form.value.price || undefined,
      quantity: form.value.quantity,
    })
    ElMessage.success(`下单成功: #${order.order_id} (${order.status})`)
  } catch (e) {
    ElMessage.error(`币安下单失败: ${e}`)
  } finally {
    submitting.value = false
  }
}

onMounted(loadBalance)
</script>

<template>
  <div class="binance-panel">
    <h3>Binance 交易面板</h3>

    <el-alert
      v-if="status"
      :type="status.connected ? 'success' : 'warning'"
      :title="status.connected ? '已连接 Binance' : '未连接 Binance'"
      :closable="false"
    />

    <el-card class="section">
      <template #header>账户余额</template>
      <el-table :data="balances" v-loading="loading" size="small">
        <el-table-column prop="asset" label="资产" />
        <el-table-column prop="free" label="可用" />
        <el-table-column prop="locked" label="锁定" />
      </el-table>
      <el-button class="mt" size="small" @click="loadBalance">刷新</el-button>
    </el-card>

    <el-card class="section">
      <template #header>下单</template>
      <el-form :model="form" label-width="90px" size="small">
        <el-form-item label="交易对">
          <el-input v-model="form.symbol" />
        </el-form-item>
        <el-form-item label="方向">
          <el-radio-group v-model="form.side">
            <el-radio-button value="Buy">买入</el-radio-button>
            <el-radio-button value="Sell">卖出</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="类型">
          <el-radio-group v-model="form.order_type">
            <el-radio-button value="Limit">限价</el-radio-button>
            <el-radio-button value="Market">市价</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item v-if="form.order_type === 'Limit'" label="价格">
          <el-input-number v-model="form.price" :min="0" :precision="2" />
        </el-form-item>
        <el-form-item label="数量">
          <el-input-number v-model="form.quantity" :min="0" :precision="6" />
        </el-form-item>
        <el-button type="primary" :loading="submitting" @click="submitOrder">
          提交下单
        </el-button>
      </el-form>
    </el-card>
  </div>
</template>

<style scoped>
.binance-panel { max-width: 720px; }
.section { margin-top: 16px; }
.mt { margin-top: 12px; }
</style>
