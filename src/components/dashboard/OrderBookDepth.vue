<template>
  <el-card class="order-book-depth" shadow="never">
    <template #header>
      <div class="card-header">
        <span class="title"><el-icon><Files /></el-icon> 订单簿</span>
        <span v-if="orderBook" class="symbol">{{ orderBook.symbol }}</span>
      </div>
    </template>

    <template v-if="orderBook && hasLevels">
      <div class="depth-columns">
        <span class="depth-col-h">价格</span>
        <span class="depth-col-h">数量</span>
      </div>

      <!-- Asks: lowest ask first (top of the ladder) -->
      <div class="depth-side asks">
        <div v-for="row in asksRows" :key="row.price" class="depth-row">
          <span class="depth-bar ask-bar" :style="{ width: `${row.pct}%` }" />
          <span class="depth-price ask">{{ row.price }}</span>
          <span class="depth-qty">{{ row.qty }}</span>
        </div>
      </div>

      <div class="depth-spread">
        <span class="spread-text">价差 {{ spread }}</span>
      </div>

      <!-- Bids: highest bid first -->
      <div class="depth-side bids">
        <div v-for="row in bidsRows" :key="row.price" class="depth-row">
          <span class="depth-bar bid-bar" :style="{ width: `${row.pct}%` }" />
          <span class="depth-price bid">{{ row.price }}</span>
          <span class="depth-qty">{{ row.qty }}</span>
        </div>
      </div>
    </template>

    <EmptyState v-else title="暂无订单簿" description="等待深度数据…" />
  </el-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Files } from '@element-plus/icons-vue'
import type { BinanceWsDepth } from '@/services/types'
import EmptyState from '@/components/common/EmptyState.vue'

interface DepthRow {
  price: string
  qty: string
  pct: number
}

const props = defineProps<{
  orderBook: BinanceWsDepth | null
}>()

const MAX_LEVELS = 10

function fmt(value: number): string {
  const n = Number(value ?? 0)
  const abs = Math.abs(n)
  const digits = abs >= 1000 ? 2 : abs >= 1 ? 4 : 8
  return n.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: digits,
  })
}

const hasLevels = computed(
  () => (props.orderBook?.bids?.length ?? 0) > 0 || (props.orderBook?.asks?.length ?? 0) > 0,
)

function maxQty(): number {
  const book = props.orderBook
  if (!book) return 1
  const all = [...book.bids, ...book.asks].map(([, q]) => Number(q ?? 0))
  return Math.max(1, ...all)
}

function toRows(levels: [number, number][]): DepthRow[] {
  const max = maxQty()
  return levels.slice(0, MAX_LEVELS).map(([price, qty]) => ({
    price: fmt(price),
    qty: fmt(qty),
    pct: Math.min(100, (Number(qty ?? 0) / max) * 100),
  }))
}

const bidsRows = computed(() => {
  const bids = props.orderBook?.bids ?? []
  const sortedDesc = [...bids].sort((a, b) => b[0] - a[0])
  return toRows(sortedDesc)
})

const asksRows = computed(() => {
  const asks = props.orderBook?.asks ?? []
  const sortedAsc = [...asks].sort((a, b) => a[0] - b[0])
  return toRows(sortedAsc)
})

const spread = computed(() => {
  const book = props.orderBook
  if (!book || !book.bids.length || !book.asks.length) return '-'
  const bestBid = book.bids[0][0]
  const bestAsk = book.asks[0][0]
  return fmt(Math.abs(bestAsk - bestBid))
})
</script>

<style scoped>
.order-book-depth :deep(.el-card__header) {
  padding: 12px 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 15px;
  font-weight: 600;
}

.symbol {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.depth-columns {
  display: flex;
  justify-content: space-between;
  padding: 4px 8px;
  font-size: 12px;
  color: var(--color-text-secondary);
}

.depth-col-h:first-child {
  width: 50%;
  text-align: left;
}

.depth-col-h:last-child {
  width: 50%;
  text-align: right;
}

.depth-side {
  position: relative;
}

.depth-row {
  position: relative;
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 20px;
  padding: 0 8px;
  font-size: 12px;
  overflow: hidden;
}

.depth-bar {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  opacity: 0.18;
}

.ask-bar {
  background: var(--color-danger);
}

.bid-bar {
  background: var(--color-success);
}

.depth-price,
.depth-qty {
  position: relative;
  z-index: 1;
}

.depth-price.ask {
  color: var(--color-danger);
}

.depth-price.bid {
  color: var(--color-success);
}

.depth-qty {
  color: var(--color-text-secondary);
}

.depth-spread {
  text-align: center;
  padding: 6px 8px;
  font-size: 12px;
  color: var(--color-text-secondary);
  border-top: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
}
</style>
