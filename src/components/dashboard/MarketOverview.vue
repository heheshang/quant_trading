<template>
  <el-row :gutter="20" style="margin-top: 20px">
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
          <span>点击刷新加载行情数据</span>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Coin, Warning } from '@element-plus/icons-vue'
import { getMarketData } from '@/services/api'

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
    const data = await getMarketData('default')
    marketData.value = data as MarketTicker
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
  gap: 16px;
}

.market-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.market-label {
  font-size: 12px;
  color: #909399;
}

.market-value {
  font-size: 16px;
  font-weight: 600;
}

.market-value.positive {
  color: #f56c6c;
}

.market-value.negative {
  color: #67c23a;
}

.market-data-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
  gap: 12px;
  color: #999;
  font-size: 14px;
}
</style>
