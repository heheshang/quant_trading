<template>
  <el-card class="positions-card">
    <template #header>
      <div class="card-header">
        <span>持仓信息</span>
      </div>
    </template>

    <el-table v-if="positions.length > 0" :data="positions" style="width: 100%">
      <el-table-column prop="symbol" label="标的代码" width="120" />
      <el-table-column prop="quantity" label="持仓数量" width="120" />
      <el-table-column prop="available_quantity" label="可用数量" width="120" />
      <el-table-column prop="avg_price" label="成本价" width="120">
        <template #default="{ row }">
          ¥{{ row.avg_price.toFixed(2) }}
        </template>
      </el-table-column>
      <el-table-column prop="market_value" label="市值" width="120">
        <template #default="{ row }">
          ¥{{ formatCurrency(row.market_value) }}
        </template>
      </el-table-column>
      <el-table-column prop="unrealized_pnl" label="浮动盈亏" width="120">
        <template #default="{ row }">
          <span :class="{ positive: row.unrealized_pnl > 0, negative: row.unrealized_pnl < 0 }">
            ¥{{ formatCurrency(row.unrealized_pnl) }}
          </span>
        </template>
      </el-table-column>
    </el-table>

    <EmptyState v-else title="暂无持仓" description="当前没有持仓信息" />
  </el-card>
</template>

<script setup lang="ts">
import EmptyState from '@/components/common/EmptyState.vue'
import { useFormatting } from '@/composables/useFormatting'
import type { Position } from '@/services/types'
import type { PropType } from 'vue'

const { formatCurrency } = useFormatting()

defineProps({
  positions: {
    type: Array as PropType<Position[]>,
    default: () => [],
  },
})
</script>

<style scoped>
.positions-card {
  margin-bottom: 20px;
}

.positive {
  color: #67C23A;
}

.negative {
  color: #F56C6C;
}
</style>
