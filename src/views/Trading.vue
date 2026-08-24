<template>
  <div class="trading-system">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>交易执行</h2>
        <div class="trade-tabs-header">
          <el-tabs v-model="activeTradeTab" class="trade-tabs" @tab-change="onTradeModeChange">
            <el-tab-pane label="纸面交易" name="paper" />
            <el-tab-pane label="实盘交易" name="live" />
            <el-tab-pane label="策略/算法" name="algorithm" />
          </el-tabs>
          <el-tag :type="tradeTabBadgeType" size="small" class="trade-tab-badge">{{ tradeTabBadge }}</el-tag>
        </div>
      </el-col>
    </el-row>

    <template v-if="activeTradeTab === 'paper'">
      <PaperAccountCard :account="paperAccount" />
      <PaperOrderForm ref="paperOrderFormRef" :strategies="strategies" :submitting="submitting" @submit="submitOrder" @reset="resetOrderForm" />
      <PositionsTable :positions="displayPositions" />
      <ActiveOrdersTable ref="activeOrdersTableRef" :orders="activeOrders" :strategies="strategies" :prices="tickerPrices" @refresh="fetchActiveOrders" @cancel="cancelOrder" />
    </template>
    <template v-else-if="activeTradeTab === 'live'">
      <AssetBalanceTable :balances="binanceBalances" title="Binance 测试网余额" @refresh="fetchAccountInfo" />
      <PaperOrderForm ref="paperOrderFormRef" :strategies="strategies" :submitting="submitting" @submit="submitOrder" @reset="resetOrderForm" />
      <PositionsTable :positions="displayPositions" />
      <ActiveOrdersTable ref="activeOrdersTableRef" :orders="activeOrders" :strategies="strategies" :prices="tickerPrices" @refresh="fetchActiveOrders" @cancel="cancelOrder" />
    </template>
    <template v-else>
      <div class="algo-tab-hint">算法单（TWAP / VWAP / Iceberg）由策略调度器产生，按 exchange='algorithm' 单独展示。</div>
      <ActiveOrdersTable ref="activeOrdersTableRef" :orders="activeOrders" :strategies="strategies" :prices="tickerPrices" @refresh="fetchActiveOrders" @cancel="cancelOrder" />
    </template>

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
import { getBinanceBalance, getBinancePositions, getBinanceTickerPrices, getLiveTrades } from '@/services/binance'
import { cancelBinanceOrder, placeBinanceOrder } from '@/services/binanceOrder'
import { useOrderStore } from '@/stores/order'
import { usePaperAccountOverview } from '@/composables/usePaperAccountOverview'
import { listenToBinanceEvents, startUserDataStream, stopUserDataStream } from '@/services/binanceWS'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type {
  AccountInfo,
  BinanceBalance,
  BinancePosition,
  BinanceWsOrderUpdate,
  LiveTrade,
  Order,
  OrderSide,
  OrderStatus,
  OrderType,
  Position,
  StrategyParams,
} from '@/services/types'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import AssetBalanceTable from '@/components/trading/AssetBalanceTable.vue'
import PaperAccountCard from '@/components/trading/PaperAccountCard.vue'
import PaperOrderForm from '@/components/trading/PaperOrderForm.vue'
import PositionsTable from '@/components/trading/PositionsTable.vue'
import ActiveOrdersTable from '@/components/trading/ActiveOrdersTable.vue'
import type { OrderFormData } from '@/components/trading/PaperOrderForm.vue'

const activeTradeTab = ref<'paper' | 'live' | 'algorithm'>('paper')
const cancelDialogVisible = ref(false)
const orderIdToCancel = ref<number | null>(null)
// 交易模式 = 当前 tab（paper/live/algorithm），供下单/取数/撤单分支使用。
const tradeMode = computed<'paper' | 'live' | 'algorithm'>(() => activeTradeTab.value)
const TRADE_TAB_META: Record<'paper' | 'live' | 'algorithm', { label: string; badgeType: string }> = {
  paper: { label: '纸面交易', badgeType: 'warning' },
  live: { label: '实盘交易', badgeType: 'success' },
  algorithm: { label: '策略/算法', badgeType: 'info' },
}
const tradeTabBadge = computed(() => TRADE_TAB_META[tradeMode.value].label)
const tradeTabBadgeType = computed(() => TRADE_TAB_META[tradeMode.value].badgeType as 'warning' | 'success' | 'info')
const binanceBalances = ref<BinanceBalance[]>([])
/** 全市场价格（供持仓市值/盈亏、订单盈亏复算，一次拉取复用）。 */
const tickerPrices = ref<Record<string, number>>({})
async function fetchTickerPrices() {
  try { tickerPrices.value = await getBinanceTickerPrices() } catch { tickerPrices.value = {} }
}
/** 本地持久化的 live 单记录（订单ID → 成交记录），供策略关联 + 真实盈亏。 */
const liveTradesMap = ref<Map<number, LiveTrade>>(new Map())
async function fetchLiveTrades() {
  try {
    const rows = await getLiveTrades()
    liveTradesMap.value = new Map(rows.map((t) => [t.order_id, t]))
  } catch {
    liveTradesMap.value = new Map()
  }
}
/** 判断是否为 Binance 限流/封禁错误（429/418/「限流」）。 */
function isRateLimitError(e: unknown): boolean {
  return /限流|Rate limited|rate limit|429|418/i.test(String((e as Error)?.message ?? e))
}

/** live 单关联的策略（Binance 不返回 strategy_id，按 order_id 本地记录本会话内的）。 */
const liveOrderStrategy = new Map<number, string>()

/** Binance「BTCUSDT」→ 展示用「BTC-USDT」；非 USDT 结尾原样返回。 */
function toDomainSymbol(sym: string): string {
  return sym.endsWith('USDT') && sym.length > 4
    ? `${sym.slice(0, -4)}-USDT`
    : sym
}
/** 由 strategy_id 解析策略名称（找不到返回空，表格显示 —）。 */
function strategyName(id: string): string {
  if (!id) return ''
  return strategies.value.find((s) => String(s.strategy_id) === String(id))?.strategy_name || ''
}
/** Binance 原始订单状态 → App 订单状态（供状态列 + 撤单按钮判断）。 */
function binanceStatusToOrderStatus(status: string): OrderStatus {
  const map: Record<string, OrderStatus> = {
    NEW: 'Submitted',
    PARTIALLY_FILLED: 'PartiallyFilled',
    FILLED: 'Filled',
    CANCELED: 'Cancelled',
    EXPIRED: 'Expired',
    REJECTED: 'Rejected',
    PENDING_CANCEL: 'Submitted',
  }
  return map[status] || 'Submitted'
}

function binancePositionToPosition(p: BinancePosition): Position {
  return {
    symbol: toDomainSymbol(p.symbol),
    quantity: p.position_amt,
    available_quantity: p.position_amt,
    avg_price: p.entry_price,
    market_value: p.notional,
    unrealized_pnl: p.un_realized_profit,
    realized_pnl: 0,
    updated_at: new Date().toISOString(),
  }
}
/** 现货持仓回退：把账户余额映射为「持仓」行（现货无衍生品持仓时展示资产）。 */
function binanceBalanceToPosition(b: BinanceBalance, price = 0, avgCost = 0): Position {
  const free = Number(b.free) || 0
  const locked = Number(b.locked) || 0
  const quantity = free + locked
  // 有成交成本则按 `(现价 - 均价) × 数量` 算浮动盈亏；无成本（未交易/测试币）按现价作参考、盈亏 0。
  return {
    symbol: b.asset,
    quantity,
    available_quantity: free,
    avg_price: avgCost || price,
    market_value: quantity * price,
    unrealized_pnl: (price - avgCost) * quantity,
    realized_pnl: 0,
    updated_at: new Date().toISOString(),
  }
}

/** 美元稳定币价格≈1；其余按 `资产+USDT` 查最新价；找不到返回 0。 */
const USD_STABLES = ['USDT', 'USDC', 'TUSD', 'BUSD', 'FDUSD', 'DAI']
function resolveBalancePrice(asset: string, prices: Record<string, number>): number {
  if (USD_STABLES.includes(asset)) return 1
  return prices[asset + 'USDT'] || 0
}

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
// 纸面持仓：由已成交订单重放得出（响应式随 paperOverview.orders 变化）。
const displayPositions = computed(() =>
  tradeMode.value === 'paper'
    ? paperOverview.account.value.holdings.map((h) => ({
        symbol: h.symbol,
        quantity: h.quantity,
        available_quantity: h.quantity,
        avg_price: h.avg_price,
        market_value: h.market_value,
        unrealized_pnl: h.unrealized_pnl,
        realized_pnl: 0,
        updated_at: new Date().toISOString(),
      }))
    : positions.value,
)
const strategies = ref<StrategyParams[]>([])
const orderStore = useOrderStore()
const submitting = ref(false)

// 纸面账户动态统计（按已成交订单 + 当前价格重放）。
const INITIAL_PAPER_CASH = 100_000
const paperOverview = usePaperAccountOverview(INITIAL_PAPER_CASH)
const paperAccount = computed(() => {
  const a = paperOverview.account.value
  return {
    account_id: 0,
    total_assets: a.totalAssets,
    available_cash: a.cash,
    frozen_cash: 0,
    market_value: a.marketValue,
    total_pnl: a.totalAssets - INITIAL_PAPER_CASH,
    daily_pnl: a.dailyPnl,
    margin: 0,
    margin_ratio: 0,
    updated_at: new Date().toISOString(),
  }
})

const paperOrderFormRef = ref<InstanceType<typeof PaperOrderForm>>()
const activeOrdersTableRef = ref<InstanceType<typeof ActiveOrdersTable>>()

const orderForm = computed<OrderFormData | undefined>(
  () => paperOrderFormRef.value?.formData,
)

function generateOrderId(): number { return Date.now() }

async function fetchAccountInfo() {
  try {
    if (tradeMode.value === 'live') {
      binanceBalances.value = await getBinanceBalance()
    } else {
      accountInfo.value = await getAccountInfo()
      await paperOverview.refresh(true)
    }
  } catch (e) {
    if (isRateLimitError(e)) {
      ElMessage.warning('Binance 限流，余额数据可能非最新')
      return
    }
    ElMessage.error('获取账户信息失败')
  }
}
async function fetchPositions() {
  try {
    if (tradeMode.value === 'live') {
      const live = (await getBinancePositions()).map(binancePositionToPosition)
      if (live.length > 0) {
        positions.value = live
      } else {
        // 现货无衍生品持仓：回退展示账户持仓（余额>0 的资产），避免视图为空。
        const balances = await getBinanceBalance()
        // 复用 onTradeModeChange 已拉取的全市场价格，补全持仓实时价格/市值。
        const prices = tickerPrices.value
        if (Object.keys(prices).length === 0) await fetchTickerPrices()
        await fetchLiveTrades()
        const held = balances.filter((b) => Number(b.free) > 0 || Number(b.locked) > 0)
        // 用本地持久化的 live 单成交记录算平均买入成本（避免逐标的查 Binance 限流）。
        const avgCostOf = (asset: string) => {
          const pair = `${asset}-USDT`
          let cost = 0
          let qty = 0
          for (const t of liveTradesMap.value.values()) {
            if (t.symbol !== pair) continue
            if (t.status !== 'FILLED' || t.side !== 'BUY') continue
            const fq = t.filled_quantity || 0
            cost += (t.price || 0) * fq
            qty += fq
          }
          return qty > 0 ? cost / qty : 0
        }
        positions.value = held.map((b): Position => {
          const price = resolveBalancePrice(b.asset, prices)
          const isStable = USD_STABLES.includes(b.asset)
          const avgCost = price > 0 && !isStable ? avgCostOf(b.asset) : (isStable ? 1 : 0)
          return binanceBalanceToPosition(b, price, avgCost)
        })
      }
    } else {
      positions.value = await getPositions()
    }
  } catch (e) {
    if (isRateLimitError(e)) {
      ElMessage.warning('Binance 限流，持仓数据可能非最新')
      return
    }
    ElMessage.error('获取持仓信息失败')
  }
}
async function fetchActiveOrders() {
  try {
    // 活跃订单展示**全部**（纸面/实盘/算法），由 ActiveOrdersTable 的「种类」下拉筛选。
    if (Object.keys(tickerPrices.value).length === 0) await fetchTickerPrices()
    await fetchLiveTrades()
    activeOrders.value = (await getActiveOrders()).map((o) => ({
      ...o,
      strategy_name: strategyName(o.strategy_id),
    }))
  } catch (e) {
    if (isRateLimitError(e)) {
      ElMessage.warning('Binance 限流，订单数据可能非最新')
      return
    }
    ElMessage.error('获取活跃订单失败')
  }
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
      // 下单带入订单类型：纸面/实盘/算法。
      exchange: tradeMode.value === 'live' ? 'live' : tradeMode.value === 'algorithm' ? 'algorithm' : 'paper',
    }
    if (tradeMode.value === 'live') {
      const placed = await placeBinanceOrder({
        symbol: order.symbol,
        side: order.side,
        order_type: order.order_type as 'Market' | 'Limit',
        price: order.price ?? undefined,
        quantity: order.quantity,
        strategy_id: formData.strategy_id,
      })
      if (placed) {
        ElMessage.success('Binance 订单提交成功: ' + placed.order_id)
        liveOrderStrategy.set(placed.order_id, formData.strategy_id || '')
        await fetchActiveOrders()
      } else {
        ElMessage.error('订单提交失败')
      }
    } else {
      const orderId = await orderStore.placeOrder(order)
      if (orderId) {
        ElMessage.success('订单提交成功: ' + orderId)
        await fetchActiveOrders()
      } else {
        ElMessage.error('订单提交失败')
      }
    }
  } catch { ElMessage.error('订单提交失败') }
  finally { submitting.value = false }
}
function resetOrderForm() {
  paperOrderFormRef.value?.resetFormData()
}
// ── Binance 用户数据流（REST 限流/封禁时的实时账户/订单补充源）──
let userDataUnlisten: UnlistenFn[] = []
function wsOrderToOrder(o: BinanceWsOrderUpdate): Order {
  return {
    order_id: o.order_id,
    strategy_id: '',
    strategy_name: '',
    // 与 REST 行一致：转换符号为域格式 + 标记为实盘单（避免被误判为纸面）。
    symbol: toDomainSymbol(o.symbol),
    order_type: (o.order_type === 'MARKET' ? 'Market' : 'Limit') as OrderType,
    side: (o.side === 'SELL' ? 'Sell' : 'Buy') as OrderSide,
    price: o.price,
    quantity: o.quantity,
    filled_quantity: o.executed_quantity,
    status: binanceStatusToOrderStatus(o.status),
    created_at: new Date(o.event_time).toISOString(),
    updated_at: new Date(o.event_time).toISOString(),
    commission: 0,
    slippage: 0,
    exchange: 'live',
  }
}
async function ensureUserDataStream() {
  try {
    await startUserDataStream()
    userDataUnlisten = await listenToBinanceEvents({
      onAccount: (p) => {
        // 实时余额：WS 推送直接写回余额卡/持仓，避免依赖被限流 REST。
        binanceBalances.value = p.balances.map((b) => ({
          asset: b.asset,
          free: b.free,
          locked: b.locked,
        }))
        fetchPositions()
      },
      onOrder: (o) => {
        // 实时订单：更新本地 live_trades 映射 + 活跃订单视图。
        const lt: LiveTrade = {
          id: 0,
          order_id: o.order_id,
          symbol: o.symbol,
          strategy_id: '',
          side: o.side,
          price: o.price,
          quantity: o.quantity,
          filled_quantity: o.executed_quantity,
          status: o.status,
          created_at: new Date(o.event_time).toISOString(),
          updated_at: new Date(o.event_time).toISOString(),
        }
        liveTradesMap.value.set(o.order_id, lt)
        const updated = wsOrderToOrder(o)
        const idx = activeOrders.value.findIndex((x) => x.order_id === o.order_id)
        if (idx >= 0) activeOrders.value[idx] = updated
        else activeOrders.value.unshift(updated)
      },
      onError: () => {},
    })
  } catch {
    // 用户流启动失败（REST 被限/未配置）→ 降级到现有 REST/live_trades 方案。
  }
}
async function teardownUserDataStream() {
  for (const u of userDataUnlisten) u()
  userDataUnlisten = []
  try { await stopUserDataStream() } catch { /* ignore */ }
}

function onTradeModeChange() {
  fetchTickerPrices()
  fetchAccountInfo()
  fetchPositions()
  fetchActiveOrders()
  if (tradeMode.value === 'live') {
    ensureUserDataStream()
  } else {
    teardownUserDataStream()
  }
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
    if (tradeMode.value === 'live' && order) {
      await cancelBinanceOrder(order.symbol, orderId)
      ElMessage.success('撤单成功')
      await fetchActiveOrders()
    } else {
      const cancelled = await cancelOrderById(orderId)
      if (cancelled) { ElMessage.success('撤单成功'); await fetchActiveOrders() }
      else ElMessage.error('撤单失败')
    }
  } catch { ElMessage.error('撤单失败') }
  finally { cancelDialogVisible.value = false; orderIdToCancel.value = null }
}

let orderEventUnlisten: Promise<() => void> | null = null
let liveOrdersUnlisten: Promise<() => void> | null = null

onMounted(() => {
  fetchTickerPrices(); fetchAccountInfo(); fetchPositions(); fetchActiveOrders(); fetchStrategies()
  orderEventUnlisten = listen('order:submitted', () => { fetchActiveOrders() })
  // 后台实盘订单监控推送：状态变化时同步本地 live_trades + 刷新活跃订单。
  liveOrdersUnlisten = listen('binance:live_orders_updated', () => {
    fetchLiveTrades()
    fetchActiveOrders()
  })
})
onUnmounted(() => {
  if (orderEventUnlisten) orderEventUnlisten.then(fn => fn())
  if (liveOrdersUnlisten) liveOrdersUnlisten.then(fn => fn())
  teardownUserDataStream()
})

// Cache-friendly: refresh account/positions/orders on re-activation.
onActivated(() => {
  fetchAccountInfo()
  fetchPositions()
  fetchActiveOrders()
})

defineExpose({
  activeTradeTab,
  tradeMode,
  binanceBalances,
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
  onTradeModeChange,
  refreshOrders,
  exportOrdersCSV,
  cancelOrder,
  confirmCancelOrder,
})
</script>

<style scoped>
.trading-system { padding: 20px; }
.header { margin-bottom: 20px; }
.trade-tabs { margin-bottom: 0; }
.trade-tabs-header { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
.trade-tab-badge { margin-left: 8px; white-space: nowrap; }
.algo-tab-hint { color: var(--color-text-secondary); padding: 8px 12px; margin-bottom: 12px; }
.balance-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}
.balance-card-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
