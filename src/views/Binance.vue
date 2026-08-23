<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import { listen } from '@tauri-apps/api/event'
import {
  getBinanceBalance,
  getBinancePositions,
  getBinanceOrders,
  checkBinanceStatus,
  startBinanceMarketData,
  stopBinanceMarketData,
  subscribeBinanceCandle,
  subscribeBinanceDepth,
} from '@/services/binance'
import { placeBinanceOrder, cancelBinanceOrder } from '@/services/binanceOrder'
import BinanceKlineChart from '@/components/trading/BinanceKlineChart.vue'
import BinanceDepthChart from '@/components/trading/BinanceDepthChart.vue'
import EmptyState from '@/components/common/EmptyState.vue'
import type {
  BinanceBalance,
  BinanceStatus,
  BinanceWsKline,
  BinanceWsDepth,
  BinancePosition,
  BinanceOrder,
} from '@/services/types'

const balances = ref<BinanceBalance[]>([])
const positions = ref<BinancePosition[]>([])
const orders = ref<BinanceOrder[]>([])
const status = ref<BinanceStatus | null>(null)
const loading = ref(false)
const submitting = ref(false)
const positionsLoading = ref(false)
const ordersLoading = ref(false)
const ordersHistory = ref(false)
const ordersSymbol = ref('BTCUSDT')

const form = ref({
  symbol: 'BTCUSDT',
  side: 'Buy' as 'Buy' | 'Sell',
  order_type: 'Limit' as 'Market' | 'Limit',
  price: 0,
  quantity: 0,
})

// ── Realtime stream ──
const wsRunning = ref(false)
const streamSymbol = ref('BTCUSDT')
const liveKlines = ref<BinanceWsKline[]>([])
const liveDepth = ref<BinanceWsDepth | null>(null)
const unlisten: Array<() => void> = []

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
    await startBinanceMarketData()
    await subscribeBinanceCandle(streamSymbol.value, '1m')
    await subscribeBinanceDepth(streamSymbol.value)
    wsRunning.value = true
  } catch (e) {
    ElMessage.error(`启动实时行情失败: ${e}`)
  }
}

async function stopStream() {
  try {
    await stopBinanceMarketData()
    wsRunning.value = false
  } catch (e) {
    ElMessage.error(`停止实时行情失败: ${e}`)
  }
}

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

async function loadPositions() {
  positionsLoading.value = true
  try {
    positions.value = await getBinancePositions()
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
  await loadBalance()
  await loadPositions()
  await loadOrders()
  unlisten.push(await listen<BinanceWsKline>('binance:kline', (ev) => {
    const existing = liveKlines.value.findIndex((k) => k.open_time === ev.payload.open_time)
    if (existing >= 0) liveKlines.value[existing] = ev.payload
    else liveKlines.value.push(ev.payload)
    if (liveKlines.value.length > 60) liveKlines.value.shift()
  }))
  unlisten.push(await listen<BinanceWsDepth>('binance:depth', (ev) => { liveDepth.value = ev.payload }))
})

onUnmounted(() => { unlisten.forEach((u) => u()) })
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
      <template #header>实时行情（WebSocket）</template>
      <div class="ws-controls">
        <el-input v-model="streamSymbol" size="small" style="width: 180px" />
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
      <template #header>持仓</template>
      <el-table :data="positions" v-loading="positionsLoading" size="small">
        <el-table-column label="交易对">
          <template #default="{ row }">{{ domainSymbol(row.symbol) }}</template>
        </el-table-column>
        <el-table-column prop="position_amt" label="数量" />
        <el-table-column prop="entry_price" label="开仓均价" />
        <el-table-column prop="mark_price" label="标记价" />
        <el-table-column label="未实现盈亏">
          <template #default="{ row }">{{ fmtNumber(row.un_realized_profit) }}</template>
        </el-table-column>
        <el-table-column prop="leverage" label="杠杆" />
        <el-table-column prop="margin_type" label="保证金模式" />
        <el-table-column prop="position_side" label="方向" />
      </el-table>
      <EmptyState v-if="!positionsLoading && positions.length === 0" title="暂无持仓" description="当前账户没有 Binance 持仓" />
      <el-button class="mt" size="small" @click="loadPositions">刷新</el-button>
    </el-card>

    <el-card class="section">
      <template #header>
        <div class="orders-header">
          <span>订单</span>
          <div class="orders-controls">
            <el-input v-model="ordersSymbol" size="small" style="width: 160px" />
            <el-radio-group v-model="ordersHistory" size="small" @change="toggleHistory">
              <el-radio-button :value="false">活动</el-radio-button>
              <el-radio-button :value="true">历史</el-radio-button>
            </el-radio-group>
            <el-button size="small" @click="loadOrders">刷新</el-button>
          </div>
        </div>
      </template>
      <el-table :data="orders" v-loading="ordersLoading" size="small">
        <el-table-column label="时间">
          <template #default="{ row }">{{ row.time ? fmtTime(row.time) : '-' }}</template>
        </el-table-column>
        <el-table-column label="交易对">
          <template #default="{ row }">{{ domainSymbol(row.symbol) }}</template>
        </el-table-column>
        <el-table-column prop="side" label="方向" />
        <el-table-column prop="order_type" label="类型" />
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
.ws-controls { display: flex; gap: 8px; margin-bottom: 12px; }
.hint { color: var(--color-text-secondary); padding: 8px 0; }
.orders-header { display: flex; justify-content: space-between; align-items: center; }
.orders-controls { display: flex; gap: 8px; align-items: center; }
</style>
