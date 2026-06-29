<template>
  <el-card class="risk-metrics-card">
    <template #header>
      <div class="card-header">
        <span>风险指标</span>
        <el-button @click="$emit('refresh')">刷新</el-button>
      </div>
    </template>

    <el-row :gutter="20">
      <el-col :span="6">
        <el-card class="risk-stat-card">
          <div class="stat-item">
            <div class="stat-label">VaR (95%)</div>
            <div class="stat-value">¥{{ formatCurrency((metrics.var_95 ?? 0) * 1000000) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card class="risk-stat-card">
          <div class="stat-item">
            <div class="stat-label">VaR (99%)</div>
            <div class="stat-value">¥{{ formatCurrency((metrics.var_99 ?? 0) * 1000000) }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card class="risk-stat-card">
          <div class="stat-item">
            <div class="stat-label">最大持仓比例</div>
            <div class="stat-value">{{ ((metrics.max_position_size ?? 0) * 100).toFixed(1) }}%</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card class="risk-stat-card">
          <div class="stat-item">
            <div class="stat-label">单日最大亏损</div>
            <div class="stat-value">{{ ((metrics.max_daily_loss ?? 0) * 100).toFixed(1) }}%</div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </el-card>
</template>

<script setup lang="ts">
defineProps<{
  metrics: Record<string, number>
}>()

defineEmits<{
  refresh: []
}>()

function formatCurrency(value: number): string {
  return value.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}
</script>

<style scoped>
.risk-metrics-card {
  margin-bottom: 20px;
}
.risk-stat-card {
  margin-bottom: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.stat-item {
  text-align: center;
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
</style>
