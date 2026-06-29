<template>
  <el-card class="active-orders-card" shadow="never">
    <template #header>
      <div class="card-header">
        <span>活跃订单</span>
        <div class="card-header-controls">
          <SearchBar v-model="searchQuery" placeholder="搜索标的/ID" />
          <el-button @click="emit('refresh')">刷新</el-button>
          <el-button size="small" @click="exportCSV">导出CSV</el-button>
        </div>
      </div>
    </template>

    <el-table v-if="paginatedOrders.length > 0" :data="paginatedOrders" style="width: 100%">
      <el-table-column prop="order_id" label="订单ID" width="200" />
      <el-table-column prop="strategy_id" label="策略" width="120" />
      <el-table-column prop="symbol" label="标的" width="120" />
      <el-table-column label="方向" width="80">
        <template #default="{ row }">
          <el-tag :type="row.side === 'Buy' ? 'success' : 'danger'" size="small">
            {{ row.side === 'Buy' ? '买入' : '卖出' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="类型" width="80">
        <template #default="{ row }">
          {{ row.order_type === 'Limit' ? '限价' : '市价' }}
        </template>
      </el-table-column>
      <el-table-column label="价格" width="100">
        <template #default="{ row }">
          <span v-if="row.price != null">¥{{ formatCurrency(row.price) }}</span>
          <span v-else>市价</span>
        </template>
      </el-table-column>
      <el-table-column prop="quantity" label="数量" width="100" />
      <el-table-column prop="filled_quantity" label="已成交" width="100" />
      <el-table-column label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getOrderStatusType(row.status)" size="small">
            {{ getOrderStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="120">
        <template #default="{ row }">
          <el-button
            size="small"
            type="danger"
            :disabled="row.status !== 'Submitted' && row.status !== 'PartiallyFilled'"
            @click="emit('cancel', row.order_id)"
          >
            撤单
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <div v-if="orders.length > 0" class="table-footer">
      <Paginator
        :total="filteredOrders.length"
        :page-size="pageSize"
        :page="currentPage"
        @update:page="onPageChange"
        @update:page-size="onPageSizeChange"
      />
    </div>
    <EmptyState v-else title="暂无活跃订单" description="当前没有活跃订单" />
  </el-card>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useFormatting } from '@/composables/useFormatting'
import { useTradingUtils } from './useTradingUtils'
import SearchBar from '@/components/common/SearchBar.vue'
import Paginator from '@/components/common/Paginator.vue'
import EmptyState from '@/components/common/EmptyState.vue'
import type { Order } from '@/services/types'

const props = withDefaults(
  defineProps<{
    orders: Order[]
  }>(),
  {
    orders: () => [],
  },
)

const emit = defineEmits<{
  refresh: []
  cancel: [orderId: number]
}>()

const { formatCurrency } = useFormatting()
const { getOrderStatusType, getOrderStatusText } = useTradingUtils()

const searchQuery = ref('')
const currentPage = ref(1)
const pageSize = ref(10)

const filteredOrders = computed(() => {
  let list = props.orders
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter(
      (o) =>
        String(o.order_id).toLowerCase().includes(q) ||
        o.symbol.toLowerCase().includes(q),
    )
  }
  return list
})

const paginatedOrders = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredOrders.value.slice(start, start + pageSize.value)
})

function onPageChange(page: number) {
  currentPage.value = page
}

function onPageSizeChange(size: number) {
  pageSize.value = size
  currentPage.value = 1
}

function exportCSV() {
  const headers = ['订单ID', '策略', '标的', '方向', '类型', '价格', '数量', '已成交', '状态']
  const rows = props.orders.map((o) => [
    o.order_id,
    o.strategy_id,
    o.symbol,
    o.side === 'Buy' ? '买入' : '卖出',
    o.order_type === 'Limit' ? '限价' : '市价',
    o.price ?? '-',
    o.quantity,
    o.filled_quantity,
    getOrderStatusText(o.status),
  ])
  const csv = [headers.join(','), ...rows.map((r) => r.join(','))].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `orders_${new Date().toISOString().slice(0, 10)}.csv`
  a.click()
  URL.revokeObjectURL(url)
}
</script>

<style scoped>
.active-orders-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.table-footer {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
