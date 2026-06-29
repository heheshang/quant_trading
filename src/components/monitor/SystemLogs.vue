<template>
  <el-row :gutter="20">
    <el-col :span="24">
      <el-card>
        <template #header>
          <div class="card-header">
            <span>系统日志</span>
            <div>
              <el-select
                :model-value="logLevel"
                @update:model-value="emit('update:logLevel', $event)"
                placeholder="日志级别"
                size="small"
              >
                <el-option label="全部" value="" />
                <el-option label="信息" value="info" />
                <el-option label="警告" value="warning" />
                <el-option label="错误" value="error" />
              </el-select>
              <el-button size="small" @click="emit('refresh')">刷新日志</el-button>
            </div>
          </div>
        </template>
        <div class="log-container">
          <div
            v-for="(log, index) in logs"
            :key="index"
            class="log-entry"
            :class="`log-${log.level}`"
          >
            <span class="log-time">[{{ formatDate(log.timestamp) }}]</span>
            <span class="log-level">[{{ log.level.toUpperCase() }}]</span>
            <span class="log-module" v-if="log.module">[{{ log.module }}]</span>
            <span class="log-message">{{ log.message }}</span>
          </div>
          <div v-if="logs.length === 0" class="no-logs">暂无日志信息</div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import type { LogEntry } from '@/services/types'

defineProps<{
  logs: LogEntry[]
  logLevel: string
}>()

const emit = defineEmits<{
  'update:logLevel': [value: string]
  refresh: []
}>()

function formatDate(dateInput: string | { timestamp: string }): string {
  let dateString: string

  if (typeof dateInput === 'string') {
    dateString = dateInput
  } else {
    dateString = dateInput.timestamp
  }

  if (dateString.endsWith('Z')) {
    return new Date(dateString).toLocaleString('zh-CN')
  } else if (dateString.includes('.')) {
    const [mainPart] = dateString.split('.')
    return new Date(mainPart + 'Z').toLocaleString('zh-CN')
  } else {
    return new Date(dateString).toLocaleString('zh-CN')
  }
}
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.log-container {
  max-height: 500px;
  overflow-y: auto;
  font-family: monospace;
}
.log-entry {
  padding: 8px 0;
  border-bottom: 1px solid #eee;
}
.log-time {
  color: #999;
  margin-right: 10px;
}
.log-level {
  margin-right: 10px;
  font-weight: bold;
}
.log-module {
  margin-right: 10px;
  color: #909399;
}
.log-info {
  color: #409EFF;
}
.log-warning {
  color: #E6A23C;
}
.log-error {
  color: #F56C6C;
}
.log-message {
  color: #333;
}
.no-logs {
  text-align: center;
  padding: 20px;
  color: #999;
}
</style>
