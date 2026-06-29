<template>
  <div class="backtest-trade-list">
    <el-table v-if="records.length > 0" :data="records" style="width: 100%">
      <el-table-column prop="date" label="日期" width="180" />
      <el-table-column prop="symbol" label="标的" width="120" />
      <el-table-column prop="type" label="类型" width="100" />
      <el-table-column prop="price" label="价格" width="120">
        <template #default="scope">
          ¥{{ formatCurrency(scope.row.price) }}
        </template>
      </el-table-column>
      <el-table-column prop="quantity" label="数量" width="100" />
      <el-table-column prop="amount" label="金额" width="120">
        <template #default="scope">
          ¥{{ formatCurrency(scope.row.amount) }}
        </template>
      </el-table-column>
      <el-table-column prop="commission" label="手续费" width="100">
        <template #default="scope">
          ¥{{ formatCurrency(scope.row.commission) }}
        </template>
      </el-table-column>
    </el-table>
    <EmptyState v-else title="暂无交易记录" description="回测完成后将在此处显示交易明细" />
  </div>
</template>

<script setup lang="ts">
import { useFormat } from '@/composables/useFormat'
import EmptyState from '@/components/common/EmptyState.vue'

export interface TradeRecord {
  date: string
  symbol: string
  type: string
  price: number
  quantity: number
  amount: number
  commission: number
}

defineProps<{
  records: TradeRecord[]
}>()

const { formatCurrency } = useFormat()
</script>

<style scoped>
.backtest-trade-list {
  width: 100%;
}
</style>
