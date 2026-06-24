<script setup lang="ts">
import { computed } from 'vue'
import { useMarketData } from '@/composables/useMarketData'
import { useFormatting } from '@/composables/useFormatting'

const props = defineProps<{
  symbol: string
}>()

const { tickerData } = useMarketData()
const { formatCurrency } = useFormatting()

const ticker = computed(() => tickerData.value[props.symbol])

const changePct = computed(() => {
  if (!ticker.value) return 0
  const last = Number(ticker.value.last)
  const open = Number(ticker.value.open24h)
  if (!open) return 0
  return ((last - open) / open) * 100
})

const isPositive = computed(() => changePct.value >= 0)
</script>

<template>
  <el-card class="realtime-ticker" shadow="never">
    <template #header>
      <div class="ticker-header">
        <span class="symbol">{{ symbol }}</span>
        <span v-if="ticker" class="timestamp">{{ new Date(Number(ticker.ts)).toLocaleString('zh-CN') }}</span>
      </div>
    </template>

    <div v-if="ticker" class="ticker-body">
      <div class="price-row">
        <span class="last-price" :class="isPositive ? 'up' : 'down'">
          {{ formatCurrency(ticker.last) }}
        </span>
        <span class="change-pct" :class="isPositive ? 'up' : 'down'">
          {{ isPositive ? '+' : '' }}{{ changePct.toFixed(2) }}%
        </span>
      </div>

      <el-descriptions :column="2" size="small" border>
        <el-descriptions-item label="24h 最高">
          {{ formatCurrency(ticker.high24h) }}
        </el-descriptions-item>
        <el-descriptions-item label="24h 最低">
          {{ formatCurrency(ticker.low24h) }}
        </el-descriptions-item>
        <el-descriptions-item label="24h 成交量">
          {{ formatCurrency(ticker.vol24h) }}
        </el-descriptions-item>
        <el-descriptions-item label="24h 开盘">
          {{ formatCurrency(ticker.open24h) }}
        </el-descriptions-item>
      </el-descriptions>
    </div>

    <el-empty v-else description="等待行情数据..." :image-size="60" />
  </el-card>
</template>

<style scoped>
.realtime-ticker {
  --ticker-up-color: #67c23a;
  --ticker-down-color: #f56c6c;
}

.ticker-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.symbol {
  font-size: 16px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.timestamp {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.ticker-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.price-row {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.last-price {
  font-size: 32px;
  font-weight: 700;
  line-height: 1.2;
}

.change-pct {
  font-size: 18px;
  font-weight: 600;
}

.up {
  color: var(--ticker-up-color);
}

.down {
  color: var(--ticker-down-color);
}
</style>
