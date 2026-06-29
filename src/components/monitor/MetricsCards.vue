<template>
  <div class="metrics-cards">
    <el-row :gutter="20">
      <el-col :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: '#409EFF' }">
              <el-icon><TrendCharts /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">总订单数</div>
              <div class="metric-value">{{ formatNumber(metrics.orders_total ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: '#67C23A' }">
              <el-icon><Check /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">已成交订单</div>
              <div class="metric-value">{{ formatNumber(metrics.orders_filled ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: '#E6A23C' }">
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

    <el-row :gutter="20" style="margin-top: 20px;">
      <el-col :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: '#F56C6C' }">
              <el-icon><Wallet /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">账户余额</div>
              <div class="metric-value">¥{{ formatCurrency(metrics.account_balance ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card class="metric-card">
          <div class="metric-header">
            <div class="metric-icon" :style="{ background: '#909399' }">
              <el-icon><Coin /></el-icon>
            </div>
            <div class="metric-info">
              <div class="metric-label">持仓价值</div>
              <div class="metric-value">¥{{ formatCurrency(metrics.position_value ?? 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
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
  color: #999;
  margin-bottom: 8px;
}
.metric-value {
  font-size: 24px;
  font-weight: bold;
  color: #333;
}
.metric-value.positive {
  color: #67C23A;
}
.metric-value.negative {
  color: #F56C6C;
}
</style>
