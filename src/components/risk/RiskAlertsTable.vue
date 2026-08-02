<template>
  <el-card class="risk-alerts-card">
    <template #header>
      <div class="card-header">
        <span>风险告警</span>
        <div class="header-actions">
          <el-select v-model="levelFilter" placeholder="级别筛选" size="small" clearable style="width:120px">
            <el-option label="严重" value="Critical" />
            <el-option label="警告" value="Warning" />
            <el-option label="信息" value="Info" />
          </el-select>
          <el-button @click="$emit('refresh')">刷新</el-button>
        </div>
      </div>
    </template>

    <el-table v-if="filteredAlerts.length > 0" :data="filteredAlerts" style="width: 100%">
      <el-table-column prop="level" label="级别" width="80">
        <template #default="scope">
          <el-tag :type="tagType(scope.row.level)">{{ tagText(scope.row.level) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="source" label="来源" width="120" />
      <el-table-column prop="message" label="消息" />
      <el-table-column prop="timestamp" label="时间" width="180">
        <template #default="scope">{{ formatDate(scope.row.timestamp) }}</template>
      </el-table-column>
      <el-table-column label="操作" width="100">
        <template #default="scope">
          <el-button
            size="small"
            type="primary"
            :disabled="scope.row.acknowledged"
            @click="$emit('acknowledge', scope.row.alert_id)"
          >
            {{ scope.row.acknowledged ? '已确认' : '确认' }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <EmptyState v-else title="暂无告警" description="当前没有风控告警" />
  </el-card>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Alert } from '@/services/types'
import EmptyState from '@/components/common/EmptyState.vue'

const props = defineProps<{
  alerts: Alert[]
}>()

defineEmits<{
  acknowledge: [alertId: number]
  refresh: []
}>()

const levelFilter = ref('')

const filteredAlerts = computed(() => {
  if (!levelFilter.value) return props.alerts
  return props.alerts.filter((a) => a.level === levelFilter.value)
})

function tagType(level: string): string {
  switch (level) {
    case 'Info': return ''
    case 'Warning': return 'warning'
    case 'Critical': return 'danger'
    default: return 'info'
  }
}

function tagText(level: string): string {
  switch (level) {
    case 'Info': return '信息'
    case 'Warning': return '警告'
    case 'Critical': return '严重'
    default: return level
  }
}

function formatDate(date: string): string {
  return new Date(date).toLocaleString('zh-CN')
}

defineExpose({ levelFilter, filteredAlerts })
</script>

<style scoped>
.risk-alerts-card {
  margin-bottom: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
