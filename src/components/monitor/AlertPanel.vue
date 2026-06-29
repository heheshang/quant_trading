<template>
  <el-row :gutter="20">
    <el-col :span="24">
      <el-card>
        <template #header>
          <div class="card-header">
            <span>最新告警</span>
            <el-button type="primary" size="small" @click="emit('refresh')">刷新告警</el-button>
          </div>
        </template>
        <el-table v-if="alerts.length > 0" :data="alerts" style="width: 100%">
          <el-table-column prop="timestamp" label="时间" width="180" />
          <el-table-column prop="source" label="来源" width="150" />
          <el-table-column prop="level" label="级别" width="100" />
          <el-table-column prop="message" label="消息" />
          <el-table-column label="操作" width="150">
            <template #default="scope">
              <el-button
                v-if="scope.row"
                size="small"
                type="primary"
                @click="emit('acknowledge', scope.row.alert_id)"
                :disabled="scope.row.acknowledged"
              >
                {{ scope.row.acknowledged ? '已确认' : '确认' }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
        <EmptyState v-else title="暂无告警" description="当前没有需要处理的告警" />
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import type { Alert } from '@/services/types'
import EmptyState from '@/components/common/EmptyState.vue'

defineProps<{
  alerts: Alert[]
}>()

const emit = defineEmits<{
  acknowledge: [alertId: number]
  refresh: []
}>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
