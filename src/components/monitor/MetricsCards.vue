<template>
  <div class="metrics-cards">
    <el-row :gutter="20">
      <el-col :xs="12" :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: 'var(--color-primary)' }">
              <el-icon><TrendCharts /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">总订单数</div>
              <div class="metric-value">{{ formatNumber(metrics.orders_total ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: 'var(--color-success)' }">
              <el-icon><Check /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">已成交订单</div>
              <div class="metric-value">{{ formatNumber(metrics.orders_filled ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: 'var(--color-warning)' }">
              <el-icon><Close /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">已撤单数</div>
              <div class="metric-value">{{ formatNumber(metrics.orders_cancelled ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20">
      <el-col :xs="12" :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: 'var(--color-danger)' }">
              <el-icon><Wallet /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">账户余额</div>
              <div class="metric-value">¥{{ formatCurrency(metrics.account_balance ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: 'var(--color-text-secondary)' }">
              <el-icon><Coin /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">持仓价值</div>
              <div class="metric-value">¥{{ formatCurrency(metrics.position_value ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: '#79BBFF' }">
              <el-icon><Trophy /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">今日盈亏</div>
              <div class="metric-value" :class="pnlClass">
                {{ pnlPrefix }}¥{{ formatCurrency(metrics.daily_pnl ?? 0) }}
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import {
  TrendCharts,
  Check,
  Close,
  Wallet,
  Coin,
  Trophy,
} from '@element-plus/icons-vue'

const props = defineProps<{
  metrics: Record<string, number>
}>()

function formatNumber(value: number): string {
  return value.toLocaleString('zh-CN')
}

function formatCurrency(value: number): string {
  return value.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

const pnlValue = computed(() => props.metrics.daily_pnl ?? 0)
const pnlClass = computed(() => ({
  positive: pnlValue.value > 0,
  negative: pnlValue.value < 0,
}))
const pnlPrefix = computed(() => (pnlValue.value > 0 ? '+' : ''))
</script>

<style scoped>
.metric-card {
  margin-bottom: 20px;
}
.metric-header {
  display: flex;
  align-items: center;
  gap: 20px;
}
.metric-icon {
  width: 60px;
  height: 60px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: #fff;
}
.metric-info {
  flex: 1;
}
.metric-label {
  font-size: 14px;
  color: var(--color-text-secondary);
  margin-bottom: 8px;
}
.metric-value {
  font-size: 24px;
  font-weight: bold;
  color: var(--color-text-primary);
}
.metric-value.positive {
  color: var(--color-success);
}
.metric-value.negative {
  color: var(--color-danger);
}
.el-row + .el-row {
  margin-top: 20px;
}
</style>
