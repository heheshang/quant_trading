<template>
  <div class="connection-status">
    <template v-if="status === 'connected'">
      <span class="status-dot dot-green"></span>
      <span class="status-text">已连接</span>
    </template>

    <template v-else-if="status === 'reconnecting'">
      <span class="status-dot dot-yellow spinning"></span>
      <span class="status-text">重连中({{ retryIn }}s)</span>
    </template>

    <template v-else>
      <span class="status-dot dot-red"></span>
      <span class="status-text">已断开</span>
      <el-button size="small" type="primary" plain @click="startListening">手动重连</el-button>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ElButton } from 'element-plus'
import { useWebSocketStatus } from '@/composables/useWebSocketStatus'

const { status, retryIn, startListening } = useWebSocketStatus()
</script>

<style scoped>
.connection-status {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 12px;
  border-radius: var(--radius-md);
  background: var(--color-bg-card);
  border: 1px solid var(--color-border-light);
  color: var(--color-text-regular);
  font-size: var(--font-size-sm);
}

.status-text {
  color: var(--color-text-regular);
  white-space: nowrap;
}

.status-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-green {
  background: #67c23a;
  box-shadow: 0 0 6px rgba(103, 194, 58, 0.5);
}

.dot-yellow {
  background: #e6a23c;
  box-shadow: 0 0 6px rgba(230, 162, 60, 0.5);
}

.dot-red {
  background: #f56c6c;
  box-shadow: 0 0 6px rgba(245, 108, 108, 0.5);
}

.spinning {
  animation: spin-breath 1.2s ease-in-out infinite;
}

@keyframes spin-breath {
  0% {
    transform: scale(0.8);
    opacity: 0.6;
  }
  50% {
    transform: scale(1.2);
    opacity: 1;
  }
  100% {
    transform: scale(0.8);
    opacity: 0.6;
  }
}

.status-text {
  color: var(--color-text-regular);
  white-space: nowrap;
}
</style>
