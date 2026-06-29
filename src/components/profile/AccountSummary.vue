<template>
  <el-card class="account-summary-card">
    <template #header>
      <div class="card-header">
        <span>账户概览</span>
      </div>
    </template>

    <div class="account-summary">
      <div class="summary-item">
        <div class="summary-label">总资产</div>
        <div class="summary-value">¥{{ formatCurrency(accountInfo.total_assets) }}</div>
      </div>

      <div class="summary-item">
        <div class="summary-label">可用资金</div>
        <div class="summary-value">¥{{ formatCurrency(accountInfo.available_cash) }}</div>
      </div>

      <div class="summary-item">
        <div class="summary-label">持仓市值</div>
        <div class="summary-value">¥{{ formatCurrency(accountInfo.market_value) }}</div>
      </div>

      <div class="summary-item">
        <div class="summary-label">当日盈亏</div>
        <div
          class="summary-value"
          :class="{ positive: accountInfo.daily_pnl > 0, negative: accountInfo.daily_pnl < 0 }"
        >
          ¥{{ formatCurrency(accountInfo.daily_pnl) }}
        </div>
      </div>

      <div class="summary-item">
        <div class="summary-label">保证金比例</div>
        <div class="summary-value">{{ (accountInfo.margin_ratio * 100).toFixed(2) }}%</div>
      </div>

      <div class="summary-item">
        <div class="summary-label">更新时间</div>
        <div class="summary-value">{{ formatDate(accountInfo.updated_at) }}</div>
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import type { AccountInfo } from '@/services/types'

defineProps<{
  accountInfo: AccountInfo
}>()

function formatCurrency(value: string | number): string {
  if (!value && value !== 0) return '0.00'
  return parseFloat(value.toString()).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

function formatDate(date: string): string {
  return new Date(date).toLocaleString('zh-CN')
}
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.account-summary {
  padding: 20px 0;
}

.summary-item {
  display: flex;
  justify-content: space-between;
  margin-bottom: 15px;
  padding-bottom: 15px;
  border-bottom: 1px solid #eee;
}

.summary-item:last-child {
  margin-bottom: 0;
  padding-bottom: 0;
  border-bottom: none;
}

.summary-label {
  font-size: 14px;
  color: #666;
}

.summary-value {
  font-size: 16px;
  font-weight: bold;
  color: #333;
}

.summary-value.positive {
  color: #67c23a;
}

.summary-value.negative {
  color: #f56c6c;
}
</style>
