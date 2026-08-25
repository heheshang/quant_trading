<template>
  <el-row :gutter="20">
    <el-col :span="24">
      <div class="dashboard-header">
        <h2 style="margin: 0">仪表盘</h2>
        <div class="header-controls">
          <el-date-picker
            :model-value="dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            size="small"
            @update:model-value="emit('update:dateRange', $event)"
            class="date-range"
          />
          <el-button type="primary" @click="emit('refresh')" :loading="loading">
            刷新数据
          </el-button>
        </div>
      </div>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
defineProps<{
  dateRange: [Date, Date]
  loading: boolean
}>()

const emit = defineEmits<{
  'update:dateRange': [value: [Date, Date]]
  refresh: []
}>()
</script>

<style scoped>
.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-sm);
}

.header-controls {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-xs);
}

.date-range {
  width: 240px;
  margin-right: 8px;
}

@media (max-width: 768px) {
  .dashboard-header {
    flex-direction: column;
    align-items: stretch;
  }

  .dashboard-header h2 {
    white-space: nowrap;
    flex-shrink: 0;
  }

  .header-controls {
    width: 100%;
  }

  .date-range {
    width: 100%;
    margin-right: 0;
  }

  .header-controls :deep(.el-button) {
    white-space: nowrap;
    flex-shrink: 0;
  }

  .header-controls :deep(.el-date-editor) {
    width: 100% !important;
  }
}
</style>
