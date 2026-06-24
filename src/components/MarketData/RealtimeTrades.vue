<script setup lang="ts">
import { computed } from 'vue'
import { useMarketData } from '@/composables/useMarketData'
import { useFormatting } from '@/composables/useFormatting'

const props = defineProps<{
  symbol: string
}>()

const { trades } = useMarketData()
const { formatCurrency, formatDate } = useFormatting()

const tradeList = computed(() => {
  const list = trades.value[props.symbol] ?? []
  return [...list].reverse()
})

function getSideType(side: string): 'success' | 'danger' {
  return side === 'buy' ? 'success' : 'danger'
}

function getSideLabel(side: string): string {
  return side === 'buy' ? '买入' : '卖出'
}
</script>

<template>
  <el-card class="realtime-trades" shadow="never">
    <template #header>
      <div class="trades-header">
        <span class="title">实时成交</span>
        <span v-if="tradeList.length" class="count">{{ tradeList.length }} 笔</span>
      </div>
    </template>

    <el-table
      v-if="tradeList.length"
      :data="tradeList"
      size="small"
      max-height="400"
      stripe
    >
      <el-table-column label="时间" min-width="140">
        <template #default="{ row }">
          {{ formatDate(row.ts) }}
        </template>
      </el-table-column>

      <el-table-column label="方向" width="70" align="center">
        <template #default="{ row }">
          <el-tag :type="getSideType(row.side)" size="small">
            {{ getSideLabel(row.side) }}
          </el-tag>
        </template>
      </el-table-column>

      <el-table-column label="价格" min-width="100" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.px) }}
        </template>
      </el-table-column>

      <el-table-column label="数量" min-width="100" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.sz) }}
        </template>
      </el-table-column>
    </el-table>

    <el-empty v-else description="等待成交数据..." :image-size="60" />
  </el-card>
</template>

<style scoped>
.realtime-trades {
  --trade-buy-color: #67c23a;
  --trade-sell-color: #f56c6c;
}

.trades-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.count {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
</style>
