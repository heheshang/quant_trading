<template>
  <el-row :gutter="20" class="header">
    <el-col :span="16">
      <h2>实时监控</h2>
    </el-col>
    <el-col :span="8" class="controls">
      <div class="status-area">
        <ConnectionStatus />
        <el-tag v-if="isPollingFallback" type="warning" size="small" class="polling-badge">
          轮询模式
        </el-tag>
        <el-button type="primary" @click="emit('refresh')" :loading="loading">刷新数据</el-button>
      </div>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import ConnectionStatus from '@/components/ws/ConnectionStatus.vue'

defineProps<{
  loading: boolean
  isPollingFallback: boolean
}>()

const emit = defineEmits<{
  refresh: []
}>()
</script>

<style scoped>
.header {
  margin-bottom: 20px;
  align-items: center;
}
.controls {
  text-align: right;
}
.status-area {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}
.polling-badge {
  flex-shrink: 0;
}
</style>
