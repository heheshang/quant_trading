<template>
  <el-card v-if="status" class="okx-status-card">
    <template #header>
      <div class="okx-status-header">
        <el-tag :type="status.connected ? 'success' : 'danger'" size="small">
          {{ status.connected ? '已连接' : '未连接' }}
        </el-tag>
        <span class="okx-status-title">OKX 交易所</span>
        <el-button size="small" type="primary" plain @click="$emit('refresh')">
          刷新
        </el-button>
      </div>
    </template>
    <el-row :gutter="20">
      <el-col :span="8">
        <div class="okx-status-item">
          <span class="label">模拟盘</span>
          <span class="value">{{ status.demo_trading ? '是' : '否' }}</span>
        </div>
      </el-col>
      <el-col :span="8">
        <div class="okx-status-item">
          <span class="label">交易所时间</span>
          <span class="value">{{ status.exchange_time || '-' }}</span>
        </div>
      </el-col>
      <el-col :span="8">
        <div class="okx-status-item">
          <span class="label">消息</span>
          <span class="value">{{ status.message || '-' }}</span>
        </div>
      </el-col>
    </el-row>
  </el-card>
</template>

<script setup lang="ts">
import { ElCard, ElTag, ElButton, ElRow, ElCol } from 'element-plus'

interface OkxStatus {
  connected?: boolean
  demo_trading?: boolean
  exchange_time?: string | null
  message?: string | null
}

defineProps<{
  status: OkxStatus | null
}>()

defineEmits<{
  refresh: []
}>()
</script>

<style scoped>
.okx-status-card {
  margin-bottom: 16px;
}

.okx-status-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.okx-status-title {
  flex: 1;
  font-weight: 600;
  font-size: 14px;
}

.okx-status-item {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
}

.okx-status-item .label {
  color: #909399;
}
</style>
