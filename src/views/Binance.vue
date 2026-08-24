<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getBinanceOrders,
  checkBinanceStatus,
  startBinanceMarketData,
  stopBinanceMarketData,
  subscribeBinanceCandle,
  subscribeBinanceDepth,
} from '@/services/binance'
import { placeBinanceOrder, cancelBinanceOrder } from '@/services/binanceOrder'
import AssetBalanceTable from '@/components/trading/AssetBalanceTable.vue'
import { useTradingUtils } from '@/components/trading/useTradingUtils'
import { getBalances, getKlines, getOrderbook, getSymbols } from '@/services/market'
import { getPositions } from '@/services/account'
import type { Position } from '@/services/types'
import BinanceKlineChart from '@/components/trading/BinanceKlineChart.vue'
import BinanceDepthChart from '@/components/trading/BinanceDepthChart.vue'
import EmptyState from '@/components/common/EmptyState.vue'
import Paginator from '@/components/common/Paginator.vue'
import type {
  BinanceBalance,
  BinanceStatus,
  BinanceWsKline,
  BinanceWsDepth,
  BinanceOrder,
  MarketDataRecord,
  OrderbookSnapshotRecord,
} from '@/services/types'

const balances = ref<BinanceBalance[]>([])
const { getOrderTypeText } = useTradingUtils()
const positions = ref<Position[]>([])
const orders = ref<BinanceOrder[]>([])
const status = ref<BinanceStatus | null>(null)
const loading = ref(false)
const submitting = ref(false)
const positionsLoading = ref(false)
const ordersLoading = ref(false)
const positionsPage = ref(1)
const positionsPageSize = ref(10)
const ordersPage = ref(1)
const ordersPageSize = ref(10)
const paginatedPositions = computed(() => {
  const start = (positionsPage.value - 1) * positionsPageSize.value
  return positions.value.slice(start, start + positionsPageSize.value)
})
const paginatedOrders = computed(() => {
  const start = (ordersPage.value - 1) * ordersPageSize.value
  return orders.value.slice(start, start + ordersPageSize.value)
})
const ordersHistory = ref(false)
const ordersSymbol = ref('BTC-USDT')
const symbols = ref<string[]>([])

const form = ref({
  symbol: 'BTC-USDT',
  side: 'Buy' as 'Buy' | 'Sell',
  order_type: 'Limit' as 'Market' | 'Limit',
  price: 0,
  quantity: 0,
})
/** 下单预计金额：限价 = 价格 × 数量；市价（无定价）显示 0。 */
const orderTotal = computed(() => {
  if (!form.value.quantity) return 0
  if (form.value.order_type === 'Limit') return (form.value.price || 0) * form.value.quantity
  return 0
})

// ── Realtime stream ──
const wsRunning = ref(false)
const streamSymbol = ref('BTC-USDT')
const liveKlines = ref<BinanceWsKline[]>([])
const liveDepth = ref<BinanceWsDepth | null>(null)
const unlisten: Array<() => void> = []
let pollTimer: ReturnType<typeof setInterval> | null = null

/** DB 行情映射：remote WS 已导入 DB，前端读 DB（removed 依赖 binance:* WS 事件）。 */
function dbKlineToWs(r: MarketDataRecord): BinanceWsKline {
  return {
    symbol: r.instrument_id,
    interval: r.timeframe,
    open_time: new Date(r.timestamp).getTime(),
    open: r.open,
    high: r.high,
    low: r.low,
    close: r.close,
    volume: r.volume,
    is_closed: true,
  }
}
function dbOrderbookToWs(r: OrderbookSnapshotRecord): BinanceWsDepth {
  return { symbol: r.symbol, bids: JSON.parse(r.bids), asks: JSON.parse(r.asks) }
}
async function pollLiveData() {
  try {
    const rows = await getKlines(streamSymbol.value, '1m', 60)
    liveKlines.value = rows.map(dbKlineToWs)
  } catch {
    // 忽略：下次轮询重试
  }
  try {
    const r = await getOrderbook(streamSymbol.value)
    liveDepth.value = r ? dbOrderbookToWs(r) : null
  } catch {
    // 忽略
  }
}

const QUOTES = ['USDT', 'USDC', 'BUSD', 'BTC', 'ETH', 'FDUSD']

/** Binance `BTCUSDT` -> domain `BTC-USDT` for display. */
function domainSymbol(sym: string): string {
  for (const q of QUOTES) {
    if (sym.endsWith(q) && sym.length > q.length) {
      return `${sym.slice(0, -q.length)}-${q}`
    }
  }
  return sym
}

function fmtTime(ms: number): string {
  return new Date(ms).toLocaleTimeString('zh-CN')
}

function fmtNumber(value: number | undefined): string {
  const n = Number(value ?? 0)
  if (!Number.isFinite(n)) return '0'
  return n.toLocaleString('zh-CN', { maximumFractionDigits: 8 })
}

function fmtKlineTime(row: BinanceWsKline): string {
  return fmtTime(row.open_time)
}

async function startStream() {
  try {
    // 启动后端 WS 并订阅（驱动 remote WS → DB 导入），显示改由 DB 轮询。
    await startBinanceMarketData()
    await subscribeBinanceCandle(streamSymbol.value, '1m')
    await subscribeBinanceDepth(streamSymbol.value)
    await pollLiveData()
    if (pollTimer) clearInterval(pollTimer)
    pollTimer = setInterval(pollLiveData, 5000)
    wsRunning.value = true
  } catch (e) {
    ElMessage.error(`启动实时行情失败: ${e}`)
  }
}

async function stopStream() {
  try {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
    await stopBinanceMarketData()
    wsRunning.value = false
  } catch (e) {
    ElMessage.error(`停止实时行情失败: ${e}`)
  }
}

async function loadBalance() {
  loading.value = true
  try {
    // 从 DB 读逐资产余额（快照写入器 60s 落库）。
    const rows = await getBalances()
    balances.value = rows.map((b) => ({ asset: b.asset, free: b.free, locked: b.locked }))
    status.value = await checkBinanceStatus()
  } catch (e) {
    ElMessage.error(`获取币安余额失败: ${e}`)
  } finally {
    loading.value = false
  }
}

async function loadPositions() {
  positionsLoading.value = true
  try {
    // 从 DB 读持仓（remote WS 导入），显示真实现货持仓。
    positions.value = await getPositions()
  } catch (e) {
    ElMessage.error(`获取币安持仓失败: ${e}`)
  } finally {
    positionsLoading.value = false
  }
}

async function loadOrders() {
  ordersLoading.value = true
  try {
    orders.value = await getBinanceOrders(ordersSymbol.value, ordersHistory.value)
  } catch (e) {
    ElMessage.error(`获取币安订单失败: ${e}`)
  } finally {
    ordersLoading.value = false
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
    await loadOrders()
  } catch (e) {
    ElMessage.error(`币安下单失败: ${e}`)
  } finally {
    submitting.value = false
  }
}

async function cancelOrder(order: BinanceOrder) {
  try {
    await cancelBinanceOrder(order.symbol, order.order_id)
    ElMessage.success(`撤单成功: #${order.order_id}`)
    await loadOrders()
  } catch (e) {
    ElMessage.error(`撤单失败: ${e}`)
  }
}

function toggleHistory() {
  loadOrders()
}

onMounted(async () => {
  try {
    symbols.value = await getSymbols()
  } catch {
    symbols.value = []
  }
  await loadBalance()
  await loadPositions()
  await loadOrders()
  // 行情显示改由 DB 轮询（startStream 时启动），不再监听 binance:kline/depth。
})

onUnmounted(() => {
  unlisten.forEach((u) => u())
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
})
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

    <AssetBalanceTable :balances="balances" title="账户余额" :loading="loading" @refresh="loadBalance" />

    <el-card class="section">
      <template #header>实时行情（WebSocket）</template>
      <div class="ws-controls">
        <el-select v-model="streamSymbol" filterable size="small" style="width: 180px">
          <el-option v-for="s in symbols" :key="s" :label="s" :value="s" />
        </el-select>
        <el-button size="small" type="primary" :disabled="wsRunning" @click="startStream">开始</el-button>
        <el-button size="small" :disabled="!wsRunning" @click="stopStream">停止</el-button>
      </div>

      <BinanceDepthChart :symbol="streamSymbol" :depth="liveDepth" />

      <el-table v-if="liveKlines.length" :data="liveKlines" size="small" max-height="240" class="mt">
        <el-table-column prop="open_time" label="时间" :formatter="fmtKlineTime" />
        <el-table-column prop="open" label="开" />
        <el-table-column prop="high" label="高" />
        <el-table-column prop="low" label="低" />
        <el-table-column prop="close" label="收" />
      </el-table>
      <div v-else-if="wsRunning" class="hint">等待 K 线流…</div>
    </el-card>

    <BinanceKlineChart />

    <el-card class="section">
      <template #header>持仓（{{ positions.length }}）</template>
      <el-table :data="paginatedPositions" v-loading="positionsLoading" size="small">
        <el-table-column label="交易对">
          <template #default="{ row }">{{ domainSymbol(row.symbol) }}</template>
        </el-table-column>
        <el-table-column prop="quantity" label="数量" />
        <el-table-column prop="avg_price" label="成本价">
          <template #default="{ row }">{{ fmtNumber(row.avg_price) }}</template>
        </el-table-column>
        <el-table-column prop="market_value" label="市值">
          <template #default="{ row }">{{ fmtNumber(row.market_value) }}</template>
        </el-table-column>
        <el-table-column label="未实现盈亏">
          <template #default="{ row }">{{ fmtNumber(row.unrealized_pnl) }}</template>
        </el-table-column>
        <el-table-column label="可用">
          <template #default="{ row }">{{ fmtNumber(row.available_quantity) }}</template>
        </el-table-column>
      </el-table>
      <EmptyState v-if="!positionsLoading && positions.length === 0" title="暂无持仓" description="当前账户没有 Binance 持仓" />
      <Paginator
        v-if="positions.length > 0"
        :total="positions.length"
        :page="positionsPage"
        :page-size="positionsPageSize"
        @update:page="positionsPage = $event"
        @update:pageSize="positionsPageSize = $event; positionsPage = 1"
      />
      <el-button class="mt" size="small" @click="loadPositions">刷新</el-button>
    </el-card>

    <el-card class="section">
      <template #header>
        <div class="orders-header">
          <span>订单（{{ orders.length }}）</span>
          <div class="orders-controls">
            <el-select v-model="ordersSymbol" filterable size="small" style="width: 160px">
              <el-option v-for="s in symbols" :key="s" :label="s" :value="s" />
            </el-select>
            <el-radio-group v-model="ordersHistory" size="small" @change="toggleHistory">
              <el-radio-button :value="false">活动</el-radio-button>
              <el-radio-button :value="true">历史</el-radio-button>
            </el-radio-group>
            <el-button size="small" @click="loadOrders">刷新</el-button>
          </div>
        </div>
      </template>
      <el-table :data="paginatedOrders" v-loading="ordersLoading" size="small">
        <el-table-column label="时间">
          <template #default="{ row }">{{ row.time ? fmtTime(row.time) : '-' }}</template>
        </el-table-column>
        <el-table-column label="交易对">
          <template #default="{ row }">{{ domainSymbol(row.symbol) }}</template>
        </el-table-column>
        <el-table-column prop="side" label="方向" />
        <el-table-column label="类型">
          <template #default="{ row }">{{ getOrderTypeText(row.order_type) }}</template>
        </el-table-column>
        <el-table-column prop="price" label="价格" />
        <el-table-column label="数量">
          <template #default="{ row }">{{ fmtNumber(row.orig_qty ?? row.executed_qty) }}</template>
        </el-table-column>
        <el-table-column prop="executed_qty" label="已成交" />
        <el-table-column prop="status" label="状态" />
        <el-table-column v-if="!ordersHistory" label="操作">
          <template #default="{ row }">
            <el-button size="small" type="danger" @click="cancelOrder(row)">撤单</el-button>
          </template>
        </el-table-column>
      </el-table>
      <EmptyState v-if="!ordersLoading && orders.length === 0" title="暂无订单" description="当前没有匹配的订单" />
      <Paginator
        v-if="orders.length > 0"
        :total="orders.length"
        :page="ordersPage"
        :page-size="ordersPageSize"
        @update:page="ordersPage = $event"
        @update:pageSize="ordersPageSize = $event; ordersPage = 1"
      />
    </el-card>

    <el-card class="section">
      <template #header>下单</template>
      <el-form :model="form" label-width="90px" size="small">
        <el-form-item label="交易对">
          <el-select v-model="form.symbol" filterable style="width: 100%">
            <el-option v-for="s in symbols" :key="s" :label="s" :value="s" />
          </el-select>
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
        <div class="order-total">
          <span v-if="form.order_type === 'Limit'">预计金额（USDT）：{{ fmtNumber(orderTotal) }}</span>
          <span v-else>市价单：按下单时实时价成交</span>
        </div>
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
.ws-controls { display: flex; gap: 8px; margin-bottom: 12px; }
.hint { color: var(--color-text-secondary); padding: 8px 0; }
.orders-header { display: flex; justify-content: space-between; align-items: center; }
.orders-controls { display: flex; gap: 8px; align-items: center; }
.order-total { color: var(--color-text-secondary); margin: 4px 0 12px; font-size: 13px; }
</style>
