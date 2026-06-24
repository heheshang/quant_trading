<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useFormatting } from '@/composables/useFormatting'
import type { WsTrade } from '@/services/types'

const props = defineProps<{
  symbol: string
}>()

const trades = ref<WsTrade[]>([])
const { formatCurrency, formatDate } = useFormatting()

let unlistenFn: UnlistenFn | null = null

onMounted(async () => {
  unlistenFn = await listen<WsTrade[]>('ws:trades', (event) => {
    const payload = event.payload
    if (!Array.isArray(payload)) return

    const filtered = payload.filter((trade) => trade.inst_id === props.symbol)
    if (filtered.length === 0) return

    trades.value.unshift(...filtered)

    if (trades.value.length > 500) {
      trades.value.splice(500)
    }
  })
})

onUnmounted(() => {
  if (unlistenFn) {
    unlistenFn()
    unlistenFn = null
  }
})

function getSideType(side: string): 'success' | 'danger' {
  return side === 'buy' ? 'success' : 'danger'
}

function getSideLabel(side: string): string {
  return side === 'buy' ? '买入' : '卖出'
}
</script>

<template>
  <div class="trade-stream">
    <div class="stream-header">
      <span class="title">实时成交</span>
      <span v-if="trades.length" class="count">{{ trades.length }} 笔</span>
    </div>

    <el-table
      v-if="trades.length"
      :data="trades"
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
  </div>
</template>

<style scoped>
.trade-stream {
  --trade-buy-color: #67c23a;
  --trade-sell-color: #f56c6c;
}

.stream-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--el-border-color-light);
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
