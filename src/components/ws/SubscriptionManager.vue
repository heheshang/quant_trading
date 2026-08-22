<template>
  <div v-loading="store.loading" class="subscription-manager">
    <el-alert
      v-if="store.error"
      :title="store.error"
      type="error"
      show-icon
      closable
      @close="store.error = null"
    />

    <div class="form-row">
      <el-select
        v-model="selectedSymbol"
        filterable
        allow-create
        default-first-option
        placeholder="选择或输入交易对"
        class="symbol-input"
      >
        <el-option
          v-for="sym in symbolOptions"
          :key="sym"
          :label="sym"
          :value="sym"
        />
      </el-select>

      <el-select
        v-model="selectedChannels"
        multiple
        placeholder="选择频道"
        class="channel-select"
      >
        <el-option label="Ticker" value="ticker" />
        <el-option label="Trades" value="trades" />
        <el-option label="Orderbook" value="orderbook" />
        <el-option label="Candle" value="candle" />
      </el-select>

      <el-button
        type="primary"
        :disabled="!canSubscribe"
        @click="handleSubscribe"
      >
        订阅
      </el-button>
    </div>

    <div class="sub-section">
      <div class="sub-header">
        <span class="sub-title">活跃订阅（{{ store.subscriptionCount }}）</span>
        <el-button text size="small" @click="store.refreshSubscriptions()">
          刷新
        </el-button>
      </div>

      <div v-if="store.subscriptions.length === 0" class="empty-state">
        暂无订阅，请在上方添加
      </div>

      <div v-else class="tag-list">
        <el-tag
          v-for="sub in store.subscriptions"
          :key="sub"
          closable
          :disable-transitions="false"
          @close="handleUnsubscribe(sub)"
        >
          {{ sub }}
        </el-tag>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useMarketDataStore } from '@/stores/marketData'

const store = useMarketDataStore()

const selectedSymbol = ref('')
const selectedChannels = ref<string[]>([])

const symbolOptions = ref([
  'BTC-USDT',
  'ETH-USDT',
  'SOL-USDT',
  'BNB-USDT',
  'DOGE-USDT',
])

const canSubscribe = computed(
  () => selectedSymbol.value.trim() !== '' && selectedChannels.value.length > 0,
)

async function handleSubscribe() {
  if (!canSubscribe.value) return
  const symbol = selectedSymbol.value.trim()
  for (const channel of selectedChannels.value) {
    await store.subscribe(symbol, channel)
  }
  selectedChannels.value = []
}

function handleUnsubscribe(key: string) {
  const [symbol, channel] = key.split(':')
  if (symbol && channel) {
    store.unsubscribe(symbol, channel)
  }
}
</script>

<style scoped>
.subscription-manager {
  padding: 16px;
}

.form-row {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 20px;
}

.symbol-input {
  width: 200px;
}

.channel-select {
  width: 260px;
  flex: 1;
}

.sub-section {
  border-top: 1px solid #ebeef5;
  padding-top: 16px;
}

.sub-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.sub-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.empty-state {
  text-align: center;
  color: var(--color-text-placeholder);
  font-size: 13px;
  padding: 32px 0;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
</style>
