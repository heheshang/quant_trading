<template>
  <el-card class="realtime-ticker-panel" shadow="never">
    <template #header>
      <div class="card-header">
        <span class="title"><el-icon><TrendCharts /></el-icon> 实时行情</span>
        <span v-if="tickers.length" class="updated-at">更新于 {{ lastUpdated }}</span>
      </div>
    </template>

    <div v-if="tickers.length" class="ticker-grid">
      <div v-for="t in tickers" :key="t.symbol" class="ticker-item">
        <div class="ticker-top">
          <span class="ticker-symbol">{{ t.symbol }}</span>
          <span class="ticker-change" :class="changeClass(t.price_change_percent)">
            {{ changePercent(t.price_change_percent) }}
          </span>
        </div>
        <div class="ticker-price">{{ formatPrice(t.last_price) }}</div>
        <div class="ticker-stats">
          <span class="ticker-stat">高 {{ formatPrice(t.high) }}</span>
          <span class="ticker-stat">低 {{ formatPrice(t.low) }}</span>
        </div>
        <div class="ticker-volume">量 {{ formatQty(t.volume) }}</div>
      </div>
    </div>

    <EmptyState v-else title="暂无行情" description="等待实时行情数据…" />
  </el-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { TrendCharts } from '@element-plus/icons-vue'
import type { BinanceWsTicker } from '@/services/types'
import EmptyState from '@/components/common/EmptyState.vue'
import { useFormatting } from '@/composables/useFormatting'

const props = defineProps<{
  tickers: BinanceWsTicker[]
}>()

const { formatNumber } = useFormatting()

function formatPrice(value: number): string {
  const abs = Math.abs(value)
  const digits = abs >= 1000 ? 2 : abs >= 1 ? 4 : 8
  return value.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: digits,
  })
}

function formatQty(value: number): string {
  return formatNumber(Number(value ?? 0))
}

function changeClass(percent: number): string {
  return percent >= 0 ? 'positive' : 'negative'
}

function changePercent(percent: number): string {
  const p = Number(percent ?? 0)
  if (!Number.isFinite(p)) return '0.00%'
  return `${p >= 0 ? '+' : ''}${p.toFixed(2)}%`
}

const lastUpdated = computed(() => {
  const t = props.tickers[0]
  if (!t || !t.event_time) return '-'
  return new Date(t.event_time).toLocaleTimeString('zh-CN')
})
</script>

<style scoped>
.realtime-ticker-panel :deep(.el-card__header) {
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

.updated-at {
  font-size: 12px;
  color: var(--color-text-secondary);
}

.ticker-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.ticker-item {
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 12px;
  background: var(--color-bg-white);
}

.ticker-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.ticker-symbol {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.ticker-change {
  font-size: 13px;
  font-weight: 600;
}

.ticker-change.positive {
  color: var(--color-success);
}

.ticker-change.negative {
  color: var(--color-danger);
}

.ticker-price {
  font-size: 20px;
  font-weight: 700;
  color: var(--color-text-primary);
  margin-bottom: 8px;
}

.ticker-stats {
  display: flex;
  gap: 12px;
  font-size: 12px;
  color: var(--color-text-secondary);
  margin-bottom: 4px;
}

.ticker-volume {
  font-size: 12px;
  color: var(--color-text-secondary);
}

@media (max-width: 768px) {
  .ticker-grid {
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  }
}
</style>
