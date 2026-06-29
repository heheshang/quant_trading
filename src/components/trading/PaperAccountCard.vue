<template>
  <el-card class="account-info-card">
    <template #header>
      <div class="card-header">
        <span>账户信息</span>
      </div>
    </template>

    <el-row :gutter="20">
      <el-col :span="6">
        <div class="account-stat">
          <div class="stat-label">总资产</div>
          <div class="stat-value">¥{{ formatCurrency(account.total_assets) }}</div>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="account-stat">
          <div class="stat-label">可用资金</div>
          <div class="stat-value">¥{{ formatCurrency(account.available_cash) }}</div>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="account-stat">
          <div class="stat-label">持仓市值</div>
          <div class="stat-value">¥{{ formatCurrency(account.market_value) }}</div>
        </div>
      </el-col>
      <el-col :span="6">
        <div class="account-stat">
          <div class="stat-label">当日盈亏</div>
          <div class="stat-value" :class="{ positive: account.daily_pnl > 0, negative: account.daily_pnl < 0 }">
            ¥{{ formatCurrency(account.daily_pnl) }}
          </div>
        </div>
      </el-col>
    </el-row>
  </el-card>
</template>

<script setup lang="ts">
import { useFormatting } from '@/composables/useFormatting'

interface PaperAccountInfo {
  account_id: number
  total_assets: number
  available_cash: number
  frozen_cash: number
  market_value: number
  total_pnl: number
  daily_pnl: number
  margin: number
  margin_ratio: number
  updated_at: Date
}

defineProps<{
  account: PaperAccountInfo
}>()

const { formatCurrency } = useFormatting()
</script>

<style scoped>
.account-stat {
  text-align: center;
  padding: 10px 0;
}

.stat-label {
  font-size: 14px;
  color: #999;
  margin-bottom: 8px;
}

.stat-value {
  font-size: 18px;
  font-weight: bold;
  color: #333;
}

.positive {
  color: #67C23A;
}

.negative {
  color: #F56C6C;
}
</style>
