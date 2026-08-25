<template>
  <el-row :gutter="20">
    <el-col :span="24">
      <el-card>
        <template #header>
          <div class="card-header">
            <span><el-icon><Coin /></el-icon> 市场价格</span>
            <el-button size="small" @click="fetchMarketData" :loading="marketLoading">刷新</el-button>
          </div>
        </template>

        <div v-if="marketError" class="market-data-placeholder">
          <el-icon><Warning /></el-icon>
          <span>{{ marketError }}</span>
        </div>

        <div v-else-if="marketLoading && !marketData" class="market-data-loading">
          <el-skeleton :rows="2" animated />
        </div>

        <div v-else-if="marketData" class="market-data-grid">
          <div class="market-item">
            <span class="market-label">标的</span>
            <span class="market-value">{{ marketData.symbol }}</span>
          </div>
          <div class="market-item">
            <span class="market-label">最新价</span>
            <span class="market-value">{{ marketData.price }}</span>
          </div>
          <div class="market-item">
            <span class="market-label">涨跌幅</span>
            <span class="market-value" :class="{ positive: (marketData.change ?? 0) >= 0, negative: (marketData.change ?? 0) < 0 }">
              {{ marketData.change_percent ?? '-' }}
            </span>
          </div>
          <div class="market-item">
            <span class="market-label">成交量</span>
            <span class="market-value">{{ marketData.volume ?? '-' }}</span>
          </div>
          <div class="market-item">
            <span class="market-label">最高价</span>
            <span class="market-value">{{ marketData.high ?? '-' }}</span>
          </div>
          <div class="market-item">
            <span class="market-label">最低价</span>
            <span class="market-value">{{ marketData.low ?? '-' }}</span>
          </div>
        </div>

        <div v-else class="market-data-placeholder">
          <el-icon><Coin /></el-icon>
          <span>暂无行情数据</span>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Coin, Warning } from '@element-plus/icons-vue'
import { getMarketData } from '@/services/market'

interface MarketTicker {
  symbol: string
  price?: number
  change?: number
  change_percent?: string
  volume?: number
  high?: number
  low?: number
}

const marketData = ref<MarketTicker | null>(null)
const marketError = ref('')
const marketLoading = ref(false)

async function fetchMarketData() {
  if (marketLoading.value) return
  marketLoading.value = true
  marketError.value = ''
  try {
    const data = await getMarketData('BTC-USDT')
    const change = Number(data.close) - Number(data.open)
    const changePct = Number(data.open) ? (change / Number(data.open)) * 100 : 0
    marketData.value = {
      symbol: data.symbol,
      price: Number(data.close),
      change,
      change_percent: `${changePct >= 0 ? '+' : ''}${changePct.toFixed(2)}%`,
      volume: Number(data.volume),
      high: Number(data.high),
      low: Number(data.low),
    }
  } catch (err: unknown) {
    marketData.value = null
    const message = err instanceof Error ? err.message : '未知错误'
    marketError.value = message.includes('Not implemented')
      ? '行情数据功能开发中'
      : '获取行情数据失败: ' + message
  } finally {
    marketLoading.value = false
  }
}

defineExpose({
  marketData,
  marketError,
  marketLoading,
  fetchMarketData,
})

onMounted(() => {
  fetchMarketData()
})
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.market-data-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-md);
}

.market-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-xxs);
}

.market-label {
  font-size: var(--font-size-xs);
  color: var(--color-text-secondary);
}

.market-value {
  font-size: var(--font-size-md);
  font-weight: 600;
}

.market-value.positive {
  color: var(--color-danger);
}

.market-value.negative {
  color: var(--color-success);
}

.market-data-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-md) 0;
  gap: var(--space-xs);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.market-data-loading {
  padding: var(--space-xs) 0;
}

@media (max-width: 768px) {
  .market-data-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
