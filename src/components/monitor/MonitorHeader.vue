<template>
  <el-row :gutter="20" class="header">
    <el-col :xs="24" :md="16">
      <h2>实时监控</h2>
    </el-col>
    <el-col :xs="24" :md="8" class="controls">
      <div class="status-area">
        <el-tag v-if="isPollingFallback" type="warning" size="small" class="polling-badge">
          轮询模式
        </el-tag>
        <el-button type="primary" @click="emit('refresh')" :loading="loading">刷新数据</el-button>
      </div>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
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
  flex-wrap: wrap;
}
.polling-badge {
  flex-shrink: 0;
}
@media (max-width: 768px) {
  .header h2 {
    white-space: nowrap;
  }
  .controls {
    text-align: left;
  }
  .status-area {
    justify-content: flex-start;
    width: 100%;
  }
}
</style>
