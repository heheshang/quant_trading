<template>
  <el-card class="positions-card">
    <template #header>
      <div class="card-header">
        <span>持仓信息（{{ filteredPositions.length }}）</span>
        <div class="card-header-controls">
          <el-select
            v-model="symbolFilter"
            filterable
            clearable
            placeholder="标的下拉搜索"
            style="width: 160px"
          >
            <el-option v-for="s in symbolOptions" :key="s" :label="s" :value="s" />
          </el-select>
          <SearchBar v-model="searchQuery" placeholder="搜索标的" />
        </div>
      </div>
    </template>

    <el-table v-if="filteredPositions.length > 0" :data="paginatedPositions" style="width: 100%">
      <el-table-column prop="symbol" label="标的代码" width="120" />
      <el-table-column prop="quantity" label="持仓数量" width="120" />
      <el-table-column prop="available_quantity" label="可用数量" width="120" />
      <el-table-column prop="avg_price" label="成本价" width="120">
        <template #default="{ row }">
          ¥{{ Number(row.avg_price || 0).toFixed(2) }}
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

    <Paginator
      v-if="filteredPositions.length > 0"
      :total="filteredPositions.length"
      :page="currentPage"
      :page-size="pageSize"
      @update:page="currentPage = $event"
      @update:pageSize="pageSize = $event; currentPage = 1"
    />
  </el-card>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import SearchBar from '@/components/common/SearchBar.vue'
import Paginator from '@/components/common/Paginator.vue'
import EmptyState from '@/components/common/EmptyState.vue'
import { useFormatting } from '@/composables/useFormatting'
import type { Position } from '@/services/types'
import type { PropType } from 'vue'

const { formatCurrency } = useFormatting()

const props = defineProps({
  positions: {
    type: Array as PropType<Position[]>,
    default: () => [],
  },
})

const searchQuery = ref('')
const symbolFilter = ref('')
const currentPage = ref(1)
const pageSize = ref(10)

/** 下拉选项：当前持仓中实际出现的标的。 */
const symbolOptions = computed(() => Array.from(new Set(props.positions.map((p) => p.symbol))))

/** 搜索 + 下拉过滤（AND）。 */
const filteredPositions = computed(() => {
  let list = props.positions
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter((p) => p.symbol.toLowerCase().includes(q))
  }
  if (symbolFilter.value) {
    list = list.filter((p) => p.symbol === symbolFilter.value)
  }
  return list
})

/** 当前页切片（最多 10 条/页）。 */
const paginatedPositions = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredPositions.value.slice(start, start + pageSize.value)
})
</script>

<style scoped>
.positions-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.card-header-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.positive {
  color: var(--color-success);
}

.negative {
  color: var(--color-danger);
}
</style>
