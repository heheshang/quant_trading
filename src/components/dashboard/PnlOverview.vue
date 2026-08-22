<template>
  <el-card class="pnl-card" shadow="never">
    <template #header>
      <div class="card-header">
        <span class="pnl-title">收益概览</span>
        <span class="realtime-indicator">
          <span class="realtime-dot" />
          <span class="realtime-text">实时</span>
        </span>
      </div>
    </template>
    <div class="pnl-content">
      <div class="pnl-item">
        <span class="pnl-label">总盈亏</span>
        <span class="pnl-value" :style="{ color: pnlColor(totalPnl) }">
          {{ totalPnl >= 0 ? '+' : '' }}{{ formatCurrency(totalPnl) }}
        </span>
      </div>
      <div class="pnl-item">
        <span class="pnl-label">未实现盈亏</span>
        <span class="pnl-value" :style="{ color: pnlColor(unrealizedPnl) }">
          {{ unrealizedPnl >= 0 ? '+' : '' }}{{ formatCurrency(unrealizedPnl) }}
        </span>
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useFormatting } from '@/composables/useFormatting'

defineProps<{
  totalPnl: number
  unrealizedPnl: number
}>()

const { formatCurrency } = useFormatting()

function pnlColor(value: number): string {
  return value >= 0 ? '#67c23a' : '#f56c6c'
}
</script>

<style scoped>
.pnl-card :deep(.el-card__header) {
  padding: 12px 16px;
}

.pnl-card :deep(.el-card__body) {
  padding: 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pnl-title {
  font-size: 16px;
  font-weight: 600;
}

.realtime-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
}

.realtime-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: #67c23a;
  animation: pulse-dot 2s ease-in-out infinite;
}

.realtime-text {
  font-size: 12px;
  color: #67c23a;
  font-weight: 500;
}

@keyframes pulse-dot {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}

.pnl-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.pnl-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pnl-label {
  font-size: 14px;
  color: var(--color-text-regular);
}

.pnl-value {
  font-size: 16px;
  font-weight: 600;
}
</style>
