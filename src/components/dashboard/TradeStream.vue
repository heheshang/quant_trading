<template>
  <el-card class="trade-stream" shadow="never">
    <template #header>
      <div class="card-header"><span class="title"><el-icon><DataLine /></el-icon> 实时成交</span></div>
    </template>

    <div v-if="trades.length" class="trade-list">
      <div v-for="(t, idx) in trades" :key="`${t.trade_time}-${idx}`" class="trade-row">
        <span class="trade-time">{{ fmtTime(t.trade_time) }}</span>
        <span class="trade-price" :class="sideClass(t)">{{ fmtPrice(t.price) }}</span>
        <span class="trade-qty">{{ fmtQty(t.quantity) }}</span>
        <span class="trade-side" :class="sideClass(t)">{{ sideLabel(t) }}</span>
      </div>
    </div>

    <EmptyState v-else title="暂无成交" description="等待实时成交数据…" />
  </el-card>
</template>

<script setup lang="ts">
import { DataLine } from '@element-plus/icons-vue'
import type { BinanceWsTrade } from '@/services/types'
import EmptyState from '@/components/common/EmptyState.vue'
import { useFormatting } from '@/composables/useFormatting'

defineProps<{
  trades: BinanceWsTrade[]
}>()

const { formatNumber } = useFormatting()

function fmtTime(ms: number): string {
  if (!ms) return '-'
  return new Date(ms).toLocaleTimeString('zh-CN', { hour12: false })
}

function fmtPrice(value: number): string {
  const n = Number(value ?? 0)
  const abs = Math.abs(n)
  const digits = abs >= 1000 ? 2 : abs >= 1 ? 4 : 8
  return n.toLocaleString('zh-CN', {
    minimumFractionDigits: digits > 4 ? 2 : 2,
    maximumFractionDigits: digits,
  })
}

function fmtQty(value: number): string {
  return formatNumber(Number(value ?? 0))
}

function isBuy(t: BinanceWsTrade): boolean {
  return !t.is_buyer_maker
}

function sideLabel(t: BinanceWsTrade): string {
  return isBuy(t) ? '买' : '卖'
}

function sideClass(t: BinanceWsTrade): string {
  return isBuy(t) ? 'buy' : 'sell'
}
</script>

<style scoped>
.trade-stream :deep(.el-card__header) {
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

.trade-list {
  max-height: 320px;
  overflow-y: auto;
}

.trade-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  font-size: 13px;
  border-bottom: 1px solid var(--color-border);
}

.trade-time {
  color: var(--color-text-secondary);
  width: 72px;
  flex-shrink: 0;
}

.trade-price {
  font-weight: 600;
  flex: 1;
  text-align: right;
}

.trade-price.buy,
.trade-side.buy {
  color: var(--color-success);
}

.trade-price.sell,
.trade-side.sell {
  color: var(--color-danger);
}

.trade-qty {
  color: var(--color-text-secondary);
  min-width: 60px;
  text-align: right;
}

.trade-side {
  width: 28px;
  text-align: center;
  font-weight: 600;
}
</style>
