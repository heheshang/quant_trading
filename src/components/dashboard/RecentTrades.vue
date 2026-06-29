<template>
  <el-row :gutter="20" style="margin-top: 20px">
    <el-col :span="24">
      <el-card>
        <template #header>
          <div class="card-header"><span>最近交易</span></div>
        </template>
        <el-table v-if="trades.length > 0" :data="trades" style="width: 100%">
          <el-table-column prop="time" label="时间" width="180" />
          <el-table-column prop="symbol" label="标的" width="120" />
          <el-table-column prop="side" label="方向" width="100">
            <template #default="scope">
              <el-tag v-if="scope?.row" :type="scope.row.side === '买入' ? 'success' : 'danger'">
                {{ scope.row.side }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="price" label="价格" />
          <el-table-column prop="quantity" label="数量" />
          <el-table-column prop="status" label="状态">
            <template #default="scope">
              <el-tag v-if="scope?.row" :type="scope.row.status === '已成交' ? 'success' : 'info'">
                {{ scope.row.status }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>
        <EmptyState v-else title="暂无交易" description="开始交易后最近交易将显示在这里" />
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import EmptyState from '@/components/common/EmptyState.vue'

interface TradeRow {
  time: string
  symbol: string
  side: string
  price: string
  quantity: string
  status: string
}

defineProps<{
  trades: TradeRow[]
}>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
