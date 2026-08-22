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

      <el-tab-pane label="OKX 交易所" name="okx">
        <OkxStatusCard :status="okxStatus" @refresh="fetchOkxStatus" />
        <OkxBalancePositions
          :balance="okxBalance" :positions="okxPositions"
          :balance-loading="okxBalanceLoading" :positions-loading="okxPositionsLoading"
          @refresh-balance="fetchOkxBalance" @refresh-positions="fetchOkxPositions"
        />
        <OkxOrderForm ref="okxOrderFormRef" :instruments="okxInstruments" :connected="okxConnected" :submitting="okxSubmitting" @submit="submitOkxOrder" />
        <el-row :gutter="20">
          <el-col :span="12">
            <OkxCandleChart ref="okxCandleChartRef" :instruments="okxInstruments" />
          </el-col>
          <el-col :span="12">
            <OkxInstrumentsPanel
              ref="okxInstrumentsPanelRef"
              :instruments="okxInstruments"
              :instruments-loading="okxInstrumentsLoading"
              :announcements="okxAnnouncements"
              :announcements-loading="okxAnnouncementsLoading"
              @refresh-instruments="fetchOkxInstruments"
              @refresh-announcements="fetchOkxAnnouncements"
            />
          </el-col>
        </el-row>
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
import { getActiveOrders } from '@/services/order'
import { getStrategies } from '@/services/strategy'
import {
  getOkxBalance, getOkxPositions, getOkxInstruments,
  checkOkxStatus, getOkxAnnouncements,
} from '@/services/okx'
import { placeOkxOrder, cancelOkxOrder } from '@/services/okxOrder'
import { useOrderStore } from '@/stores/order'
import type {
  AccountInfo,
  OkxAnnouncementPage,
  OkxBalance,
  OkxConnectionStatus,
  OkxInstrument,
  OkxPlaceOrderRequest,
  OkxPosition,
  Order,
  Position,
  StrategyParams,
} from '@/services/types'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import PaperAccountCard from '@/components/trading/PaperAccountCard.vue'
import PaperOrderForm from '@/components/trading/PaperOrderForm.vue'
import PositionsTable from '@/components/trading/PositionsTable.vue'
import ActiveOrdersTable from '@/components/trading/ActiveOrdersTable.vue'
import OkxStatusCard from '@/components/trading/OkxStatusCard.vue'
import OkxBalancePositions from '@/components/trading/OkxBalancePositions.vue'
import OkxOrderForm from '@/components/trading/OkxOrderForm.vue'
import OkxCandleChart from '@/components/trading/OkxCandleChart.vue'
import OkxInstrumentsPanel from '@/components/trading/OkxInstrumentsPanel.vue'
import type { OrderFormData } from '@/components/trading/PaperOrderForm.vue'
import type { OkxOrderFormData } from '@/components/trading/OkxOrderForm.vue'

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

// OKX state
const okxStatus = ref<OkxConnectionStatus | null>(null)
const okxBalance = ref<OkxBalance[]>([])
const okxPositions = ref<OkxPosition[]>([])
const okxInstruments = ref<OkxInstrument[]>([])
const okxInstrumentsLoading = ref(false)
const okxAnnouncements = ref<OkxAnnouncementPage[]>([])
const okxAnnouncementsLoading = ref(false)
const okxBalanceLoading = ref(false)
const okxPositionsLoading = ref(false)
const okxConnected = computed(() => okxStatus.value?.connected === true)
const okxSubmitting = ref(false)
const paperOrderFormRef = ref<InstanceType<typeof PaperOrderForm>>()
const activeOrdersTableRef = ref<InstanceType<typeof ActiveOrdersTable>>()
const okxCandleChartRef = ref<InstanceType<typeof OkxCandleChart>>()
const okxOrderFormRef = ref<InstanceType<typeof OkxOrderForm>>()
const okxInstrumentsPanelRef = ref<InstanceType<typeof OkxInstrumentsPanel>>()

const orderForm = computed<OrderFormData | undefined>(
  () => paperOrderFormRef.value?.formData,
)
const okxOrderForm = computed<OkxOrderFormData | undefined>(
  () => okxOrderFormRef.value?.formData,
)
const okxCandleError = computed(
  () => okxCandleChartRef.value?.candleError ?? '',
)
const showAllInstruments = computed({
  get: () => okxInstrumentsPanelRef.value?.showAll ?? false,
  set: (value: boolean) => {
    if (okxInstrumentsPanelRef.value) {
      okxInstrumentsPanelRef.value.showAll = value
    }
  },
})

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

async function fetchOkxCandles() {
  return okxCandleChartRef.value?.fetchCandles()
}

function cancelOrder(orderId: number) { orderIdToCancel.value = orderId; cancelDialogVisible.value = true }
async function confirmCancelOrder() {
  const orderId = orderIdToCancel.value
  if (orderId === null) return
  const order = activeOrders.value.find((o) => o.order_id === orderId)
  if (!order) { ElMessage.error('未找到对应订单'); cancelDialogVisible.value = false; orderIdToCancel.value = null; return }
  try {
    const cancelled = await cancelOkxOrder(order.symbol, orderId.toString())
    if (cancelled) { ElMessage.success('撤单成功'); await fetchActiveOrders() }
    else ElMessage.error('撤单失败')
  } catch { ElMessage.error('撤单失败') }
  finally { cancelDialogVisible.value = false; orderIdToCancel.value = null }
}

// OKX fetch
async function fetchOkxStatus() {
  try { okxStatus.value = await checkOkxStatus() } catch { /* ignore */ }
}
async function fetchOkxBalance() {
  okxBalanceLoading.value = true
  try { okxBalance.value = await getOkxBalance() } catch { ElMessage.error('获取OKX余额失败') }
  finally { okxBalanceLoading.value = false }
}
async function fetchOkxPositions() {
  okxPositionsLoading.value = true
  try { okxPositions.value = await getOkxPositions() } catch { ElMessage.error('获取OKX持仓失败') }
  finally { okxPositionsLoading.value = false }
}
async function fetchOkxInstruments() {
  okxInstrumentsLoading.value = true
  try { okxInstruments.value = await getOkxInstruments() } catch { ElMessage.error('获取交易对失败') }
  finally { okxInstrumentsLoading.value = false }
}
async function fetchOkxAnnouncements() {
  okxAnnouncementsLoading.value = true
  try { okxAnnouncements.value = await getOkxAnnouncements() } catch { ElMessage.error('获取公告失败') }
  finally { okxAnnouncementsLoading.value = false }
}
async function submitOkxOrder(orderData: OkxOrderFormData = okxOrderForm.value ?? {
  instId: '',
  side: 'buy',
  ordType: 'limit',
  px: 0,
  sz: 0,
}) {
  if (!orderData) return
  okxSubmitting.value = true
  try {
    const request: OkxPlaceOrderRequest = {
      instId: orderData.instId,
      tdMode: 'cash',
      side: orderData.side,
      ordType: orderData.ordType,
      sz: String(orderData.sz),
      ...(orderData.px > 0 ? { px: String(orderData.px) } : {}),
    }
    const result = await placeOkxOrder(request)
    ElMessage.success('OKX 订单提交成功: ' + result.ordId)
    fetchOkxBalance(); fetchOkxPositions()
  } catch (err: unknown) {
    ElMessage.error('OKX 下单失败: ' + (err instanceof Error ? err.message : '未知错误'))
  }
  finally { okxSubmitting.value = false }
}

let orderEventUnlisten: Promise<() => void> | null = null

onMounted(() => {
  fetchAccountInfo(); fetchPositions(); fetchActiveOrders(); fetchStrategies()
  fetchOkxStatus(); fetchOkxInstruments(); fetchOkxAnnouncements()
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
  fetchOkxStatus()
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
  okxStatus,
  okxBalance,
  okxPositions,
  okxInstruments,
  okxInstrumentsLoading,
  okxAnnouncements,
  okxAnnouncementsLoading,
  okxBalanceLoading,
  okxPositionsLoading,
  okxConnected,
  okxSubmitting,
  okxOrderFormRef,
  okxOrderForm,
  okxCandleChartRef,
  okxCandleError,
  showAllInstruments,
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
  fetchOkxStatus,
  fetchOkxBalance,
  fetchOkxPositions,
  fetchOkxInstruments,
  fetchOkxAnnouncements,
  fetchOkxCandles,
  submitOkxOrder,
})
</script>

<style scoped>
.trading-system { padding: 20px; }
.header { margin-bottom: 20px; }
.trade-tabs { margin-bottom: 20px; }
</style>
