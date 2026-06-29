<template>
  <el-row :gutter="20">
    <!-- 账户余额 -->
    <el-col :span="12">
      <el-card class="okx-section-card">
        <template #header>
          <div class="card-header">
            <span>账户余额</span>
            <el-button size="small" @click="$emit('refreshBalance')">刷新</el-button>
          </div>
        </template>
        <el-table :data="balance" size="small" style="width: 100%" v-loading="balanceLoading">
          <el-table-column prop="ccy" label="币种" width="60" />
          <el-table-column prop="cashBal" label="余额" width="100">
            <template #default="{ row }">
              {{ formatCurrency(row.cashBal) }}
            </template>
          </el-table-column>
          <el-table-column prop="eq" label="总权益" width="100">
            <template #default="{ row }">
              {{ formatCurrency(row.eq) }}
            </template>
          </el-table-column>
          <el-table-column prop="uTime" label="更新时间" width="140">
            <template #default="{ row }">
              {{ formatTimestamp(row.uTime) }}
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </el-col>

    <!-- 持仓 -->
    <el-col :span="12">
      <el-card class="okx-section-card">
        <template #header>
          <div class="card-header">
            <span>持仓</span>
            <el-button size="small" @click="$emit('refreshPositions')">刷新</el-button>
          </div>
        </template>
        <el-table :data="positions" size="small" style="width: 100%" v-loading="positionsLoading">
          <el-table-column prop="instId" label="产品" width="100" />
          <el-table-column prop="pos" label="数量" width="80" />
          <el-table-column prop="avgPx" label="均价" width="100" />
          <el-table-column prop="upl" label="未实现盈亏" width="100">
            <template #default="{ row }">
              <span :class="{ positive: Number(row.upl) > 0 }">{{ row.upl }}</span>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { ElCard, ElButton, ElTable, ElTableColumn, ElRow, ElCol } from 'element-plus'
import { useFormatting } from '@/composables/useFormatting'

const { formatCurrency } = useFormatting()

defineProps<{
  balance: unknown[]
  positions: unknown[]
  balanceLoading: boolean
  positionsLoading: boolean
}>()

defineEmits<{
  refreshBalance: []
  refreshPositions: []
}>()

function formatTimestamp(ts: string): string {
  if (!ts || ts === '0') return '-'
  return new Date(Number(ts)).toLocaleString('zh-CN')
}
</script>

<style scoped>
.okx-section-card {
  margin-bottom: 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.positive {
  color: #67C23A;
}
</style>
