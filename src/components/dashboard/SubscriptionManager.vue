<template>
  <el-card class="subscription-manager" shadow="never">
    <template #header>
      <div class="card-header"><span class="title"><el-icon><Connection /></el-icon> 订阅管理</span></div>
    </template>

    <div class="status-row">
      <span class="status-dot" :class="statusClass" />
      <span class="status-text">{{ statusLabel }}</span>
      <el-tag size="small" :type="running ? 'success' : 'info'" effect="plain">
        {{ running ? '运行中' : '已停止' }}
      </el-tag>
    </div>

    <div v-if="symbols.length" class="symbols-chips">
      <span class="chips-label">标的</span>
      <el-tag v-for="s in symbols" :key="s" size="small" effect="plain" class="symbol-chip">
        {{ s }}
      </el-tag>
    </div>

    <div class="actions">
      <el-button size="small" type="primary" :disabled="running" @click="$emit('start')">开始</el-button>
      <el-button size="small" :disabled="!running" @click="$emit('stop')">停止</el-button>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Connection } from '@element-plus/icons-vue'

const props = defineProps<{
  running: boolean
  status: string
  symbols: string[]
}>()

defineEmits<{
  start: []
  stop: []
}>()

const statusLabelMap: Record<string, string> = {
  idle: '未连接',
  connecting: '连接中',
  connected: '已连接',
  disconnected: '已断开',
  error: '连接异常',
}

const statusLabel = computed(() => statusLabelMap[props.status] ?? props.status)

const statusClass = computed(() => {
  if (props.status === 'connected') return 'on'
  if (props.status === 'connecting' || props.status === 'disconnected') return 'pending'
  return 'off'
})
</script>

<style scoped>
.subscription-manager :deep(.el-card__header) {
  padding: 12px 16px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 15px;
  font-weight: 600;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--color-text-secondary);
}

.status-dot.on {
  background: var(--color-success);
}

.status-dot.pending {
  background: var(--color-warning);
}

.status-dot.off {
  background: var(--color-text-secondary);
}

.status-text {
  font-size: 14px;
  font-weight: 500;
}

.symbols-chips {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}

.chips-label {
  font-size: 12px;
  color: var(--color-text-secondary);
  margin-right: 4px;
}

.actions {
  display: flex;
  gap: 8px;
}
</style>
