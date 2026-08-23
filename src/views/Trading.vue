<template>
  <div class="trading-system">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>交易执行</h2>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTradeTab" class="trade-tabs">
      <el-tab-pane label="模拟交易" name="paper">
        <PaperAccountCard :account="accountInfo" />
        <PaperOrderForm ref="paperOrderFormRef" :strategies="strategies" :submitting="submitting" @submit="submitOrder" @reset="resetOrderForm" />
        <PositionsTable :positions="positions" />
        <ActiveOrdersTable ref="activeOrdersTableRef" :orders="activeOrders" @refresh="fetchActiveOrders" @cancel="cancelOrder" />
      </el-tab-pane>
    </el-tabs>

    <ConfirmDialog
      v-model:visible="cancelDialogVisible"
      title="确认撤单"
      message="确定要撤销此订单吗？"
      type="warning"
      confirm-text="撤单"
      @confirm="confirmCancelOrder"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onActivated } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { ElMessage } from 'element-plus'
import { getAccountInfo, getPositions } from '@/services/account'
import { cancelOrder as cancelOrderById, getActiveOrders } from '@/services/order'
import { getStrategies } from '@/services/strategy'
import { useOrderStore } from '@/stores/order'
import type {
  AccountInfo,
  Order,
  Position,
  StrategyParams,
} from '@/services/types'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import PaperAccountCard from '@/components/trading/PaperAccountCard.vue'
import PaperOrderForm from '@/components/trading/PaperOrderForm.vue'
import PositionsTable from '@/components/trading/PositionsTable.vue'
import ActiveOrdersTable from '@/components/trading/ActiveOrdersTable.vue'
import type { OrderFormData } from '@/components/trading/PaperOrderForm.vue'

const activeTradeTab = ref('paper')
const cancelDialogVisible = ref(false)
const orderIdToCancel = ref<number | null>(null)

// Paper trading state
const accountInfo = ref<AccountInfo>({
  account_id: 0,
  total_assets: 0,
  available_cash: 0,
  frozen_cash: 0,
  market_value: 0,
  total_pnl: 0,
  daily_pnl: 0,
  margin: 0,
  margin_ratio: 0,
  updated_at: new Date().toISOString(),
})
const positions = ref<Position[]>([])
const activeOrders = ref<Order[]>([])
const strategies = ref<StrategyParams[]>([])
const orderStore = useOrderStore()
const submitting = ref(false)

const paperOrderFormRef = ref<InstanceType<typeof PaperOrderForm>>()
const activeOrdersTableRef = ref<InstanceType<typeof ActiveOrdersTable>>()

const orderForm = computed<OrderFormData | undefined>(
  () => paperOrderFormRef.value?.formData,
)

function generateOrderId(): number { return Date.now() }

async function fetchAccountInfo() {
  try { accountInfo.value = await getAccountInfo() } catch { ElMessage.error('获取账户信息失败') }
}
async function fetchPositions() {
  try { positions.value = await getPositions() } catch { ElMessage.error('获取持仓信息失败') }
}
async function fetchActiveOrders() {
  try { activeOrders.value = await getActiveOrders() } catch { ElMessage.error('获取活跃订单失败') }
}
async function fetchStrategies() {
  try { strategies.value = await getStrategies() } catch { ElMessage.error('获取策略列表失败') }
}

async function submitOrder(formData: OrderFormData = orderForm.value ?? {
  strategy_id: '',
  symbol: '',
  side: 'Buy',
  order_type: 'Limit',
  price: 0,
  quantity: 0,
}) {
  if (!formData) return
  submitting.value = true
  try {
    const order: Order = {
      order_id: generateOrderId(), strategy_id: formData.strategy_id,
      symbol: formData.symbol, order_type: formData.order_type, side: formData.side,
      price: formData.order_type === 'Limit' ? formData.price : null,
      quantity: formData.quantity, filled_quantity: 0, status: 'Pending',
      created_at: new Date().toISOString(), updated_at: new Date().toISOString(),
      commission: 0, slippage: 0,
    }
    const orderId = await orderStore.placeOrder(order)
    if (orderId) {
      ElMessage.success('订单提交成功: ' + orderId)
      await fetchActiveOrders()
    } else ElMessage.error('订单提交失败')
  } catch { ElMessage.error('订单提交失败') }
  finally { submitting.value = false }
}
function resetOrderForm() {
  paperOrderFormRef.value?.resetFormData()
}

function refreshOrders() {
  return fetchActiveOrders()
}

function exportOrdersCSV() {
  activeOrdersTableRef.value?.exportCSV()
}

function cancelOrder(orderId: number) { orderIdToCancel.value = orderId; cancelDialogVisible.value = true }
async function confirmCancelOrder() {
  const orderId = orderIdToCancel.value
  if (orderId === null) return
  const order = activeOrders.value.find((o) => o.order_id === orderId)
  if (!order) { ElMessage.error('未找到对应订单'); cancelDialogVisible.value = false; orderIdToCancel.value = null; return }
  try {
    const cancelled = await cancelOrderById(orderId)
    if (cancelled) { ElMessage.success('撤单成功'); await fetchActiveOrders() }
    else ElMessage.error('撤单失败')
  } catch { ElMessage.error('撤单失败') }
  finally { cancelDialogVisible.value = false; orderIdToCancel.value = null }
}

let orderEventUnlisten: Promise<() => void> | null = null

onMounted(() => {
  fetchAccountInfo(); fetchPositions(); fetchActiveOrders(); fetchStrategies()
  orderEventUnlisten = listen('order:submitted', () => { fetchActiveOrders() })
})
onUnmounted(() => {
  if (orderEventUnlisten) orderEventUnlisten.then(fn => fn())
})

// Cache-friendly: refresh account/positions/orders on re-activation.
onActivated(() => {
  fetchAccountInfo()
  fetchPositions()
  fetchActiveOrders()
})

defineExpose({
  activeTradeTab,
  cancelDialogVisible,
  orderIdToCancel,
  accountInfo,
  positions,
  activeOrders,
  strategies,
  submitting,
  orderForm,
  fetchAccountInfo,
  fetchPositions,
  fetchActiveOrders,
  fetchStrategies,
  submitOrder,
  resetOrderForm,
  refreshOrders,
  exportOrdersCSV,
  cancelOrder,
  confirmCancelOrder,
})
</script>

<style scoped>
.trading-system { padding: 20px; }
.header { margin-bottom: 20px; }
.trade-tabs { margin-bottom: 20px; }
</style>
