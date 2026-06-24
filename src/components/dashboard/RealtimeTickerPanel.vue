<script setup lang="ts">
import { computed } from 'vue'
import { useMarketData } from '@/composables/useMarketData'
import { useFormatting } from '@/composables/useFormatting'
import type { WsTicker } from '@/services/types'

const props = defineProps<{
  symbols: string[]
}>()

const { tickerData } = useMarketData()
const { formatCurrency } = useFormatting()

function getPricePrecision(value: string | undefined): number {
  if (!value) return 2
  const str = value.trim()
  const dotIndex = str.indexOf('.')
  if (dotIndex === -1) return 0
  return Math.min(str.length - dotIndex - 1, 8)
}

function formatPrice(value: string | undefined): string {
  if (!value) return '-'
  const precision = getPricePrecision(value)
  return parseFloat(value).toFixed(precision)
}

function computeChangePct(ticker: WsTicker | undefined): number {
  if (!ticker) return 0
  const last = Number(ticker.last)
  const open = Number(ticker.open24h)
  if (!open || open === 0) return 0
  return ((last - open) / open) * 100
}

const tickerList = computed(() => {
  return props.symbols.map((symbol) => {
    const ticker = tickerData.value[symbol]
    const changePct = computeChangePct(ticker)
    return {
      symbol,
      ticker,
      changePct,
      isPositive: changePct >= 0,
    }
  })
})
</script>

<template>
  <el-card class="realtime-ticker-panel" shadow="never">
    <template #header>
      <div class="card-header">
        <span>实时行情</span>
      </div>
    </template>

    <el-empty
      v-if="!symbols || symbols.length === 0"
      description="无持仓交易对"
      :image-size="60"
    />

    <div v-else class="ticker-list">
      <div class="ticker-header-row">
        <div class="col-symbol">交易对</div>
        <div class="col-price">最新价</div>
        <div class="col-change">24h 涨跌</div>
        <div class="col-high-low">24h 高/低</div>
      </div>

      <div
        v-for="item in tickerList"
        :key="item.symbol"
        class="ticker-row"
      >
        <div class="col-symbol">
          <span class="symbol-name">{{ item.symbol }}</span>
        </div>
        <div
          class="col-price"
          :class="item.isPositive ? 'up' : 'down'"
        >
          {{ item.ticker ? formatPrice(item.ticker.last) : '-' }}
        </div>
        <div class="col-change">
          <span
            class="change-badge"
            :class="item.isPositive ? 'up' : 'down'"
          >
            {{ item.isPositive ? '+' : '' }}{{ item.changePct.toFixed(2) }}%
          </span>
        </div>
        <div class="col-high-low">
          <div class="high-low-row">
            <span class="label">高:</span>
            <span class="value">{{ item.ticker ? formatCurrency(item.ticker.high24h) : '-' }}</span>
          </div>
          <div class="high-low-row">
            <span class="label">低:</span>
            <span class="value">{{ item.ticker ? formatCurrency(item.ticker.low24h) : '-' }}</span>
          </div>
        </div>
      </div>
    </div>
  </el-card>
</template>

<style scoped>
.realtime-ticker-panel {
  --ticker-up-color: #f56c6c;
  --ticker-down-color: #67c23a;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.ticker-list {
  display: flex;
  flex-direction: column;
}

.ticker-header-row {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  background-color: var(--el-fill-color-light);
  border-bottom: 1px solid var(--el-border-color-lighter);
  font-size: 12px;
  color: var(--el-text-color-secondary);
  font-weight: 500;
}

.ticker-row {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  transition: background-color 0.2s ease;
}

.ticker-row:hover {
  background-color: var(--el-fill-color-light);
}

.ticker-row:last-child {
  border-bottom: none;
}

.col-symbol {
  flex: 1.5;
  min-width: 0;
}

.col-price {
  flex: 1.5;
  min-width: 0;
  text-align: right;
  font-family: var(--el-font-family-monospace, monospace);
  font-weight: 600;
  font-size: 14px;
}

.col-change {
  flex: 1;
  min-width: 0;
  text-align: right;
}

.change-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 13px;
  font-weight: 600;
}

.change-badge.up {
  background-color: #f56c6c;
  color: #ffffff;
}

.change-badge.down {
  background-color: #67c23a;
  color: #ffffff;
}

.col-high-low {
  flex: 2;
  min-width: 0;
  text-align: right;
  font-size: 12px;
  color: var(--el-text-color-regular);
}

.symbol-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.high-low-row {
  display: inline-flex;
  align-items: center;
}

.high-low-row + .high-low-row {
  margin-left: 8px;
}

.label {
  color: var(--el-text-color-secondary);
  margin-right: 2px;
}

.value {
  font-family: var(--el-font-family-monospace, monospace);
}

.up {
  color: var(--ticker-up-color);
}

.down {
  color: var(--ticker-down-color);
}
</style>
