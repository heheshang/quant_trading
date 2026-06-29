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
            style="width: 240px; margin-right: 8px"
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
  margin-bottom: 20px;
}

.header-controls {
  display: flex;
  align-items: center;
}
</style>
