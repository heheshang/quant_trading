<template>
  <div class="trading-system">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>交易执行</h2>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTradeTab" class="trade-tabs">
      <!-- 模拟交易 Tab -->
      <el-tab-pane label="模拟交易" name="paper">
        <!-- 账户信息 -->
        <el-card class="account-info-card">
          <template #header>
            <div class="card-header">
              <span>账户信息</span>
            </div>
          </template>
          
          <el-row :gutter="20">
            <el-col :span="6">
              <div class="account-stat">
                <div class="stat-label">总资产</div>
                <div class="stat-value">¥{{ formatCurrency(accountInfo.total_assets) }}</div>
              </div>
            </el-col>
            <el-col :span="6">
              <div class="account-stat">
                <div class="stat-label">可用资金</div>
                <div class="stat-value">¥{{ formatCurrency(accountInfo.available_cash) }}</div>
              </div>
            </el-col>
            <el-col :span="6">
              <div class="account-stat">
                <div class="stat-label">持仓市值</div>
                <div class="stat-value">¥{{ formatCurrency(accountInfo.market_value) }}</div>
              </div>
            </el-col>
            <el-col :span="6">
              <div class="account-stat">
                <div class="stat-label">当日盈亏</div>
                <div class="stat-value" :class="{ positive: accountInfo.daily_pnl > 0, negative: accountInfo.daily_pnl < 0 }">
                  ¥{{ formatCurrency(accountInfo.daily_pnl) }}
                </div>
              </div>
            </el-col>
          </el-row>
        </el-card>

        <!-- 订单创建 -->
        <el-card class="order-form-card">
          <template #header>
            <div class="card-header">
              <span>创建订单</span>
            </div>
          </template>
          
          <el-form :model="orderForm" label-width="100px" :rules="orderRules" ref="orderFormRef">
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="策略" prop="strategy_id">
                  <el-select v-model="orderForm.strategy_id" placeholder="选择策略" style="width: 100%">
                    <el-option 
                      v-for="strategy in strategies" 
                      :key="strategy.strategy_id" 
                      :label="strategy.strategy_name" 
                      :value="strategy.strategy_id" 
                    />
                  </el-select>
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="标的代码" prop="symbol">
                  <el-input v-model="orderForm.symbol" placeholder="输入标的代码，如 600519.SH" />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="买卖方向" prop="side">
                  <el-select v-model="orderForm.side" placeholder="选择买卖方向" style="width: 100%">
                    <el-option label="买入" value="Buy" />
                    <el-option label="卖出" value="Sell" />
                  </el-select>
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="订单类型" prop="order_type">
                  <el-select v-model="orderForm.order_type" placeholder="选择订单类型" style="width: 100%">
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
                    v-model="orderForm.price" 
                    :min="0" 
                    :precision="2" 
                    :step="0.01" 
                    style="width: 100%" 
                    :disabled="orderForm.order_type === 'Market'"
                  />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="数量" prop="quantity">
                  <el-input-number 
                    v-model="orderForm.quantity" 
                    :min="0" 
                    :precision="2" 
                    :step="100" 
                    style="width: 100%" 
                  />
                </el-form-item>
              </el-col>
            </el-row>
            
            <el-form-item>
              <el-button type="primary" @click="submitOrder" :loading="submitting">提交订单</el-button>
              <el-button @click="resetOrderForm">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <!-- 持仓信息 -->
        <el-card class="positions-card">
          <template #header>
            <div class="card-header">
              <span>持仓信息</span>
            </div>
          </template>
          
          <el-table v-if="positions.length > 0" :data="positions" style="width: 100%">
            <el-table-column prop="symbol" label="标的代码" width="120" />
            <el-table-column prop="quantity" label="持仓数量" width="120" />
            <el-table-column prop="available_quantity" label="可用数量" width="120" />
            <el-table-column prop="avg_price" label="成本价" width="120">
              <template #default="scope">
                ¥{{ scope.row.avg_price.toFixed(2) }}
              </template>
            </el-table-column>
            <el-table-column prop="market_value" label="市值" width="120">
              <template #default="scope">
                ¥{{ formatCurrency(scope.row.market_value) }}
              </template>
            </el-table-column>
            <el-table-column prop="unrealized_pnl" label="浮动盈亏" width="120">
              <template #default="scope">
                <span :class="{ positive: scope.row.unrealized_pnl > 0, negative: scope.row.unrealized_pnl < 0 }">
                  ¥{{ formatCurrency(scope.row.unrealized_pnl) }}
                </span>
              </template>
            </el-table-column>
          </el-table>
          <EmptyState v-else title="暂无持仓" description="当前没有持仓信息" />
        </el-card>

        <!-- 活跃订单 -->
        <el-card class="active-orders-card">
          <template #header>
            <div class="card-header">
              <span>活跃订单</span>
              <div class="card-header-controls">
                <SearchBar v-model="orderSearchQuery" placeholder="搜索标的/ID" @search="onOrderSearch" />
                <el-button @click="refreshOrders">刷新</el-button>
                <el-button size="small" @click="exportOrdersCSV">导出CSV</el-button>
              </div>
            </div>
          </template>
          
          <el-table v-if="paginatedOrders.length > 0" :data="paginatedOrders" style="width: 100%">
            <el-table-column prop="order_id" label="订单ID" width="200" />
            <el-table-column prop="strategy_id" label="策略" width="120" />
            <el-table-column prop="symbol" label="标的" width="120" />
            <el-table-column prop="side" label="方向" width="80">
              <template #default="scope">
                <el-tag :type="scope.row.side === 'Buy' ? 'success' : 'danger'">
                  {{ scope.row.side === 'Buy' ? '买入' : '卖出' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="order_type" label="类型" width="80">
              <template #default="scope">
                {{ scope.row.order_type === 'Limit' ? '限价' : '市价' }}
              </template>
            </el-table-column>
            <el-table-column prop="price" label="价格" width="100">
              <template #default="scope">
                <span v-if="scope.row.price">¥{{ scope.row.price.toFixed(2) }}</span>
                <span v-else>市价</span>
              </template>
            </el-table-column>
            <el-table-column prop="quantity" label="数量" width="100" />
            <el-table-column prop="filled_quantity" label="已成交" width="100" />
            <el-table-column prop="status" label="状态" width="100">
              <template #default="scope">
                <el-tag :type="getOrderStatusType(scope.row.status)">
                  {{ getOrderStatusText(scope.row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120">
              <template #default="scope">
                <el-button 
                  size="small" 
                  type="danger" 
                  @click="cancelOrder(scope.row.order_id)"
                  :disabled="scope.row.status !== 'Submitted' && scope.row.status !== 'PartiallyFilled'"
                >
                  撤单
                </el-button>
              </template>
            </el-table-column>
          </el-table>
          <div v-if="activeOrders.length > 0" class="table-footer">
            <Paginator
              :total="filteredOrders.length"
              :page-size="orderPageSize"
              :current-page="orderCurrentPage"
              @update:current-page="orderCurrentPage = $event"
              @update:page-size="orderPageSize = $event"
            />
          </div>
          <EmptyState v-else title="暂无活跃订单" description="当前没有活跃订单" />
        </el-card>
      </el-tab-pane>

      <!-- OKX 交易所 Tab -->
      <el-tab-pane label="OKX 交易所" name="okx">
        <!-- OKX 连接状态 -->
        <el-card class="okx-status-card" v-if="okxStatus">
          <template #header>
            <div class="card-header">
              <span><el-tag :type="okxStatus.connected ? 'success' : 'danger'" size="small">
                {{ okxStatus.connected ? '已连接' : '未连接' }}
              </el-tag> OKX 交易所</span>
              <el-button size="small" @click="fetchOkxStatus">刷新状态</el-button>
            </div>
          </template>
          <el-row :gutter="20">
            <el-col :span="8"><div class="okx-status-item"><span class="label">模拟盘</span><span>{{ okxStatus.demo_trading ? '是' : '否' }}</span></div></el-col>
            <el-col :span="8"><div class="okx-status-item"><span class="label">交易所时间</span><span>{{ okxStatus.exchange_time || '-' }}</span></div></el-col>
            <el-col :span="8"><div class="okx-status-item"><span class="label">消息</span><span>{{ okxStatus.message || '-' }}</span></div></el-col>
          </el-row>
        </el-card>

        <el-row :gutter="20">
          <!-- OKX 账户余额 -->
          <el-col :span="12">
            <el-card class="okx-section-card">
              <template #header>
                <div class="card-header"><span>账户余额</span><el-button size="small" @click="fetchOkxBalance">刷新</el-button></div>
              </template>
              <el-table :data="okxBalance" size="small" style="width: 100%" v-loading="okxBalanceLoading">
                <el-table-column prop="ccy" label="币种" width="60" />
                <el-table-column prop="cashBal" label="余额" width="100" />
                <el-table-column prop="eq" label="总权益" width="100" />
                <el-table-column prop="uTime" label="更新时间" width="140">
                  <template #default="scope">{{ formatTimestamp(scope.row.uTime) }}</template>
                </el-table-column>
              </el-table>
            </el-card>
          </el-col>

          <!-- OKX 持仓 -->
          <el-col :span="12">
            <el-card class="okx-section-card">
              <template #header>
                <div class="card-header"><span>持仓</span><el-button size="small" @click="fetchOkxPositions">刷新</el-button></div>
              </template>
              <el-table :data="okxPositions" size="small" style="width: 100%" v-loading="okxPositionsLoading">
                <el-table-column prop="instId" label="产品" width="100" />
                <el-table-column prop="pos" label="数量" width="80" />
                <el-table-column prop="avgPx" label="均价" width="100" />
                <el-table-column prop="upl" label="未实现盈亏" width="100">
                  <template #default="scope"><span :class="{ positive: Number(scope.row.upl) > 0 }">{{ scope.row.upl }}</span></template>
                </el-table-column>
              </el-table>
            </el-card>
          </el-col>
        </el-row>

        <!-- OKX 下单表单 -->
        <el-card class="okx-section-card">
          <template #header>
            <div class="card-header"><span>OKX 下单</span></div>
          </template>
          <el-form :model="okxOrderForm" label-width="100px" :rules="okxOrderRules" ref="okxOrderFormRef" inline>
            <el-form-item label="交易对" prop="instId">
              <el-select v-model="okxOrderForm.instId" placeholder="选择交易对" style="width:160px" filterable>
                <el-option v-for="inst in okxInstruments" :key="inst.instId" :label="inst.instId" :value="inst.instId" />
              </el-select>
            </el-form-item>
            <el-form-item label="方向" prop="side">
              <el-select v-model="okxOrderForm.side" style="width:120px">
                <el-option label="买入" value="buy" />
                <el-option label="卖出" value="sell" />
              </el-select>
            </el-form-item>
            <el-form-item label="类型" prop="ordType">
              <el-select v-model="okxOrderForm.ordType" style="width:120px">
                <el-option label="限价" value="limit" />
                <el-option label="市价" value="market" />
              </el-select>
            </el-form-item>
            <el-form-item label="价格" prop="px">
              <el-input-number v-model="okxOrderForm.px" :min="0" :precision="2" :step="0.01" style="width:160px" />
            </el-form-item>
            <el-form-item label="数量" prop="sz">
              <el-input-number v-model="okxOrderForm.sz" :min="0" :precision="4" :step="0.001" style="width:160px" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="submitOkxOrder" :loading="okxSubmitting" :disabled="!okxConnected">提交</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-row :gutter="20">
          <!-- OKX K线 -->
          <el-col :span="12">
            <el-card class="okx-section-card">
              <template #header>
                <div class="card-header"><span>K 线</span></div>
              </template>
              <el-form inline size="small">
                <el-form-item label="交易对">
                  <el-select v-model="okxCandleInstId" style="width:160px" filterable @change="fetchOkxCandles">
                    <el-option v-for="inst in okxInstruments" :key="inst.instId" :label="inst.instId" :value="inst.instId" />
                  </el-select>
                </el-form-item>
                <el-form-item label="周期">
                  <el-select v-model="okxCandleBar" style="width:100px" @change="fetchOkxCandles">
                    <el-option label="1m" value="1m" /><el-option label="5m" value="5m" />
                    <el-option label="15m" value="15m" /><el-option label="1H" value="1H" />
                    <el-option label="4H" value="4H" /><el-option label="1D" value="1D" />
                  </el-select>
                </el-form-item>
              </el-form>
              <div ref="okxCandleChartRef" style="height: 300px"></div>
              <div v-if="okxCandleError" class="market-data-placeholder">{{ okxCandleError }}</div>
            </el-card>
          </el-col>

          <!-- OKX 交易对列表 + 公告 -->
          <el-col :span="12">
            <el-card class="okx-section-card">
              <template #header>
                <div class="card-header"><span>交易对列表</span><el-button size="small" @click="fetchOkxInstruments">刷新</el-button></div>
              </template>
              <el-table :data="okxInstruments.slice(0, 10)" size="small" style="width: 100%" max-height="200" v-loading="okxInstrumentsLoading">
                <el-table-column prop="instId" label="产品ID" width="120" />
                <el-table-column prop="baseCcy" label="基础币" width="80" />
                <el-table-column prop="quoteCcy" label="计价币" width="80" />
                <el-table-column prop="instType" label="类型" width="80" />
              </el-table>
              <div v-if="okxInstruments.length > 10" class="show-more" @click="showAllInstruments = !showAllInstruments">
                {{ showAllInstruments ? '收起' : `显示全部 (${okxInstruments.length})` }}
              </div>
            </el-card>
            <el-card class="okx-section-card" style="margin-top: 12px;">
              <template #header>
                <div class="card-header"><span>OKX 公告</span><el-button size="small" @click="fetchOkxAnnouncements" :loading="okxAnnouncementsLoading">刷新</el-button></div>
              </template>
              <div v-if="okxAnnouncements.length === 0" class="market-data-placeholder">暂无公告</div>
              <ul v-else class="announcement-list">
                <li v-for="(item, idx) in okxAnnouncements.slice(0, 5)" :key="idx">
                  <a :href="item.url" target="_blank" rel="noopener"># {{ item.title || item.notice }}</a>
                </li>
              </ul>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>
    </el-tabs>

    <!-- Cancel order confirm dialog -->
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
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { ElMessage, type FormInstance } from 'element-plus';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import EmptyState from '@/components/common/EmptyState.vue';
import SearchBar from '@/components/common/SearchBar.vue';
import Paginator from '@/components/common/Paginator.vue';
import {
  getAccountInfo, getPositions, getActiveOrders, getStrategies,
  getOkxBalance, getOkxPositions, placeOkxOrder, getOkxCandles,
  getOkxInstruments, checkOkxStatus, getOkxAnnouncements,
  cancelOkxOrder,
} from '@/services/api';
import { useOrderStore } from '@/stores/order';
import * as echarts from 'echarts';

// ========== Tabs ==========
const activeTradeTab = ref('paper');

// ========== Confirm dialog state ==========
const cancelDialogVisible = ref(false);
const orderIdToCancel = ref<number | null>(null);

// ==================== Orders search / pagination / export ====================
const orderSearchQuery = ref('')
const orderCurrentPage = ref(1)
const orderPageSize = ref(10)

const filteredOrders = computed(() => {
  let list = activeOrders.value
  if (orderSearchQuery.value) {
    const q = orderSearchQuery.value.toLowerCase()
    list = list.filter((o: any) => o.order_id?.toLowerCase().includes(q) || o.symbol?.toLowerCase().includes(q))
  }
  return list
})

const paginatedOrders = computed(() => {
  const start = (orderCurrentPage.value - 1) * orderPageSize.value
  return filteredOrders.value.slice(start, start + orderPageSize.value)
})

function onOrderSearch() { orderCurrentPage.value = 1 }

function exportOrdersCSV() {
  const headers = ['订单ID', '策略', '标的', '方向', '类型', '价格', '数量', '已成交', '状态']
  const rows = activeOrders.value.map((o: any) => [
    o.order_id, o.strategy_id, o.symbol,
    o.side === 'Buy' ? '买入' : '卖出',
    o.order_type === 'Limit' ? '限价' : '市价',
    o.price ?? '-', o.quantity, o.filled_quantity,
    getOrderStatusText(o.status),
  ])
  const csv = [headers.join(','), ...rows.map((r: string[]) => r.join(','))].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url; a.download = `orders_${new Date().toISOString().slice(0, 10)}.csv`
  a.click(); URL.revokeObjectURL(url)
}

// ========== Paper trading state ==========
const accountInfo = ref({
  account_id: 0,
  total_assets: 0,
  available_cash: 0,
  frozen_cash: 0,
  market_value: 0,
  total_pnl: 0,
  daily_pnl: 0,
  margin: 0,
  margin_ratio: 0,
  updated_at: new Date()
});

const positions = ref<any[]>([]);
const activeOrders = ref<any[]>([]);
const strategies = ref<any[]>([]);

const orderForm = ref({
  strategy_id: '',
  symbol: '600519.SH',
  side: 'Buy',
  order_type: 'Limit',
  price: 1685.00,
  quantity: 100
});

const orderRules = {
  strategy_id: [{ required: true, message: '请选择策略', trigger: 'change' }],
  symbol: [{ required: true, message: '请输入标的代码', trigger: 'blur' }],
  side: [{ required: true, message: '请选择买卖方向', trigger: 'change' }],
  order_type: [{ required: true, message: '请选择订单类型', trigger: 'change' }],
  price: [{ required: true, message: '请输入价格', trigger: 'blur' }],
  quantity: [{ required: true, message: '请输入数量', trigger: 'blur' }]
};

const orderFormRef = ref<FormInstance>();
const submitting = ref(false);
const orderStore = useOrderStore();

// ========== OKX state ==========
const okxStatus = ref<any>(null)
const okxBalance = ref<any[]>([])
const okxPositions = ref<any[]>([])
const okxInstruments = ref<any[]>([])
const okxInstrumentsLoading = ref(false)
const okxAnnouncements = ref<any[]>([])
const okxAnnouncementsLoading = ref(false)
const okxBalanceLoading = ref(false)
const okxPositionsLoading = ref(false)
const okxConnected = computed(() => okxStatus.value?.connected === true)

const okxOrderForm = ref({ instId: '', side: 'buy', ordType: 'limit', px: 0, sz: 0 })
const okxOrderFormRef = ref<FormInstance>()
const okxSubmitting = ref(false)

const okxOrderRules = {
  instId: [{ required: true, message: '请选择交易对', trigger: 'change' }],
  side: [{ required: true, message: '请选择方向', trigger: 'change' }],
  ordType: [{ required: true, message: '请选择类型', trigger: 'change' }],
}

const okxCandleInstId = ref('BTC-USDT')
const okxCandleBar = ref('1H')
const okxCandleChartRef = ref<HTMLDivElement>()
const okxCandleError = ref('')
let okxCandleChart: echarts.ECharts | null = null

const showAllInstruments = ref(false)

function formatTimestamp(ts: string): string {
  if (!ts || ts === '0') return '-'
  const d = new Date(Number(ts))
  return d.toLocaleString('zh-CN')
}

// ========== OKX fetch functions ==========
async function fetchOkxStatus() {
  try {
    okxStatus.value = await checkOkxStatus()
  } catch { /* ignore */ }
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
async function fetchOkxCandles() {
  okxCandleError.value = ''
  try {
    const candles = await getOkxCandles(okxCandleInstId.value, okxCandleBar.value, 60)
    await nextTick()
    renderOkxCandleChart(candles)
  } catch (err: any) {
    okxCandleError.value = err?.message || '获取K线失败'
  }
}
function renderOkxCandleChart(candles: any[]) {
  if (!okxCandleChartRef.value || !candles.length) return
  if (!okxCandleChart) okxCandleChart = echarts.init(okxCandleChartRef.value)
  okxCandleChart.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'cross' } },
    xAxis: { type: 'category', data: candles.map((c: any) => c.ts || c[0]).reverse(), axisLabel: { rotate: 45, fontSize: 10 } },
    yAxis: { type: 'value', scale: true, splitNumber: 6 },
    series: [{
      type: 'candlestick', data: candles.map((c: any) => {
        const ohl = c.o || c[1], cl = c.c || c[4], hi = c.h || c[2], lo = c.l || c[3]
        return [ohl, cl, lo, hi]
      }).reverse(),
      itemStyle: { color: '#67C23A', color0: '#F56C6C', borderColor: '#67C23A', borderColor0: '#F56C6C' },
    }],
    grid: { left: '5%', right: '5%', bottom: '15%', top: '5%' },
  })
}
async function fetchOkxAnnouncements() {
  okxAnnouncementsLoading.value = true
  try {
    const raw: any = await getOkxAnnouncements()
    okxAnnouncements.value = Array.isArray(raw) ? raw : raw?.data ? raw.data : []
  } catch { ElMessage.error('获取公告失败') }
  finally { okxAnnouncementsLoading.value = false }
}
async function submitOkxOrder() {
  if (!okxOrderFormRef.value) return
  await okxOrderFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    okxSubmitting.value = true
    try {
      const result = await placeOkxOrder({ instId: okxOrderForm.value.instId, side: okxOrderForm.value.side as 'Buy' | 'Sell', ordType: okxOrderForm.value.ordType, sz: okxOrderForm.value.sz, px: okxOrderForm.value.px } as any)
      ElMessage.success(`OKX 订单提交成功: ${result.ordId}`)
      fetchOkxBalance()
      fetchOkxPositions()
    } catch (err: any) {
      ElMessage.error('OKX 下单失败: ' + (err?.message || ''))
    } finally { okxSubmitting.value = false }
  })
}

// Format currency
function formatCurrency(value: any): string {
  if (!value) return '0.00';
  return parseFloat(value.toString()).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

// Get order status type for tag
function getOrderStatusType(status: string): string {
  switch (status) {
    case 'Pending':
      return '';
    case 'Submitted':
      return 'primary';
    case 'PartiallyFilled':
      return 'warning';
    case 'Filled':
      return 'success';
    case 'Cancelled':
      return 'danger';
    default:
      return 'info';
  }
}

// Get order status text
function getOrderStatusText(status: string): string {
  switch (status) {
    case 'Pending':
      return '待提交';
    case 'Submitted':
      return '已提交';
    case 'PartiallyFilled':
      return '部分成交';
    case 'Filled':
      return '已成交';
    case 'Cancelled':
      return '已撤单';
    default:
      return status;
  }
}

// Generate a simple order ID using timestamp
function generateOrderId(): number {
  return Date.now();
}

// Fetch account info
async function fetchAccountInfo() {
  try {
    accountInfo.value = await getAccountInfo() as any;
  } catch (error) {
    console.error('Failed to fetch account info:', error);
    ElMessage.error('获取账户信息失败');
  }
}

// Fetch positions
async function fetchPositions() {
  try {
    positions.value = await getPositions() as any;
  } catch (error) {
    console.error('Failed to fetch positions:', error);
    ElMessage.error('获取持仓信息失败');
  }
}

// Fetch active orders
async function fetchActiveOrders() {
  try {
    activeOrders.value = await getActiveOrders() as any;
  } catch (error) {
    console.error('Failed to fetch active orders:', error);
    ElMessage.error('获取活跃订单失败');
  }
}

// Fetch strategies
async function fetchStrategies() {
  try {
    strategies.value = await getStrategies() as any;
  } catch (error) {
    console.error('Failed to fetch strategies:', error);
    ElMessage.error('获取策略列表失败');
  }
}

// Submit order
async function submitOrder() {
  if (!orderFormRef.value) return;

  await orderFormRef.value.validate(async (valid) => {
    if (!valid) return;

    submitting.value = true;
    try {
      const order = {
        order_id: generateOrderId(),
        strategy_id: orderForm.value.strategy_id,
        symbol: orderForm.value.symbol,
        order_type: orderForm.value.order_type,
        side: orderForm.value.side,
        price: orderForm.value.order_type === 'Limit' ? orderForm.value.price : null,
        quantity: orderForm.value.quantity,
        filled_quantity: 0,
        status: 'Pending',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        commission: 0,
        slippage: 0
      };

      const orderId = await orderStore.placeOrder(order as any);
      if (orderId) {
        ElMessage.success(`订单提交成功: ${orderId}`);
        resetOrderForm();
        await fetchActiveOrders();
      } else {
        ElMessage.error('订单提交失败');
      }
    } catch (error) {
      console.error('Failed to submit order:', error);
      ElMessage.error('订单提交失败: ' + (error as Error).message);
    } finally {
      submitting.value = false;
    }
  });
}

// Cancel order — show ConfirmDialog first
function cancelOrder(orderId: number) {
  orderIdToCancel.value = orderId;
  cancelDialogVisible.value = true;
}

async function confirmCancelOrder() {
  const orderId = orderIdToCancel.value;
  if (orderId === null) return;

  // Find the order's symbol from activeOrders
  const order = activeOrders.value.find((o: any) => o.order_id === orderId);
  if (!order) {
    ElMessage.error('未找到对应订单');
    cancelDialogVisible.value = false;
    orderIdToCancel.value = null;
    return;
  }

  try {
    const cancelled = await cancelOkxOrder(order.symbol, orderId.toString());
    if (cancelled) {
      ElMessage.success('撤单成功');
      await fetchActiveOrders();
    } else {
      ElMessage.error('撤单失败');
    }
  } catch (error) {
    console.error('Failed to cancel order:', error);
    ElMessage.error('撤单失败');
  } finally {
    cancelDialogVisible.value = false;
    orderIdToCancel.value = null;
  }
}

// Reset order form
function resetOrderForm() {
  orderForm.value = {
    strategy_id: '',
    symbol: '600519.SH',
    side: 'Buy',
    order_type: 'Limit',
    price: 1685.00,
    quantity: 100
  };
}

// Refresh orders
async function refreshOrders() {
  await fetchActiveOrders();
  ElMessage.success('刷新成功');
}

// Initialize on mount
onMounted(() => {
  fetchAccountInfo();
  fetchPositions();
  fetchActiveOrders();
  fetchStrategies();

  // Fetch OKX data
  fetchOkxStatus();
  fetchOkxInstruments();
  fetchOkxAnnouncements();

  // Listen for order:submitted events from backend
  const unlisten = listen('order:submitted', (event) => {
    const data = event.payload as Record<string, unknown>;
    console.log('Order submitted event received:', data);
    ElMessage.success(`订单已提交: ${data.symbol}`);
    fetchActiveOrders();
  });
  // Store unlisten for cleanup
  (window as any).__order_event_unlisten = unlisten;
});

onUnmounted(() => {
  const unlisten = (window as any).__order_event_unlisten;
  if (unlisten) {
    unlisten.then((fn: () => void) => fn());
  }
});
</script>

<style scoped>
.trading-system {
  padding: 20px;
}

.header {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.table-footer {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.account-info-card {
  margin-bottom: 20px;
}

.account-stat {
  text-align: center;
  padding: 10px 0;
}

.stat-label {
  font-size: 14px;
  color: #999;
  margin-bottom: 8px;
}

.stat-value {
  font-size: 18px;
  font-weight: bold;
  color: #333;
}

.stat-value.positive {
  color: #67C23A;
}

.stat-value.negative {
  color: #F56C6C;
}

.order-form-card {
  margin-bottom: 20px;
}

.positions-card {
  margin-bottom: 20px;
}

.active-orders-card {
  margin-bottom: 20px;
}

.trade-tabs {
  margin-bottom: 20px;
}

.okx-status-card {
  margin-bottom: 16px;
}

.okx-section-card {
  margin-bottom: 16px;
}

.okx-status-item {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
}

.okx-status-item .label {
  color: #909399;
}

.market-data-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 20px;
  color: #909399;
  font-size: 14px;
}

.show-more {
  text-align: center;
  padding: 8px;
  color: #409EFF;
  cursor: pointer;
  font-size: 13px;
}

.show-more:hover {
  text-decoration: underline;
}

.announcement-list {
  list-style: none;
  margin: 0;
  padding: 4px 12px;
}

.announcement-list li {
  padding: 4px 0;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.announcement-list a {
  color: #606266;
  text-decoration: none;
}

.announcement-list a:hover {
  color: #409EFF;
  text-decoration: underline;
}
</style>