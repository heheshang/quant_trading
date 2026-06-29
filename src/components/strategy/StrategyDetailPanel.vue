<template>
  <el-card class="strategy-detail-panel" shadow="hover">
    <template #header>
      <div class="panel-header">
        <el-icon class="header-icon"><Operation /></el-icon>
        <span>策略详情</span>
        <el-tag :type="display.type" size="small" class="status-tag">{{ display.label }}</el-tag>
      </div>
    </template>

    <div class="panel-content">
      <el-descriptions :column="2" border class="info-descriptions">
        <el-descriptions-item label="策略ID">{{ strategyId }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ formatDate(createTime) }}</el-descriptions-item>
        <el-descriptions-item label="最后更新">{{ formatDate(updateTime) }}</el-descriptions-item>
        <el-descriptions-item label="策略类型">{{ strategyType }}</el-descriptions-item>
        <el-descriptions-item v-if="instanceLabel" label="实例标签">
          {{ instanceLabel }}
        </el-descriptions-item>
      </el-descriptions>

      <div class="section">
        <h4 class="section-title">策略描述</h4>
        <p class="description-text">{{ description }}</p>
      </div>

      <div class="section">
        <h4 class="section-title">标签</h4>
        <div class="tags-container">
          <el-tag
            v-for="tag in tags"
            :key="tag"
            size="small"
            effect="plain"
            class="tag-item"
          >{{ tag }}</el-tag>
        </div>
      </div>

      <div class="section">
        <h4 class="section-title">交易标的</h4>
        <div class="symbols-container">
          <el-tag
            v-for="(sym, idx) in symbols"
            :key="idx"
            size="small"
            effect="plain"
            class="symbol-item"
          >{{ sym }}</el-tag>
          <span v-if="!symbols.length" class="empty-hint">无</span>
        </div>
      </div>

      <div class="section">
        <h4 class="section-title">策略参数</h4>
        <div v-if="Object.keys(paramsValues).length" class="params-list">
          <div v-for="(val, key) in paramsValues" :key="key" class="param-row">
            <span class="param-key">{{ key }}</span>
            <span class="param-val">{{ formatParamValue(val) }}</span>
          </div>
        </div>
        <span v-else class="empty-hint">无自定义参数</span>
      </div>

    </div>

    <template #footer>
      <div class="panel-footer">
        <el-button size="small" @click="handleEdit">编辑策略</el-button>
        <el-button size="small" type="primary" :disabled="isRunning" @click="handleStart">启动</el-button>
        <el-button size="small" type="danger" :disabled="!isRunning" @click="handleStop">停止</el-button>
        <el-button size="small" @click="handleRefresh">刷新</el-button>
      </div>
    </template>
  </el-card>
</template>

<script setup lang="ts">
import { toRef } from 'vue'
import type { StrategyStatus } from '@/services/types'
import { useStrategyStatus } from '@/composables/useStrategyStatus'

const props = withDefaults(
  defineProps<{
    strategyId: string
    status: StrategyStatus
    description?: string
    tags?: string[]
    symbols?: string[]
    paramsValues?: Record<string, unknown>
    createTime?: number
    updateTime?: number
    strategyType?: string
    isRunning?: boolean
    instanceLabel?: string
  }>(),
  {
    description: '暂无策略描述',
    tags: () => [],
    symbols: () => [],
    paramsValues: () => ({}),
    createTime: () => Date.now(),
    updateTime: () => Date.now(),
    strategyType: 'custom',
    isRunning: false,
    instanceLabel: '',
  }
)

const emit = defineEmits<{
  'edit': []
  'start': []
  'stop': []
  'refresh': []
}>()

const display = useStrategyStatus(toRef(props, 'status'))

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function formatParamValue(val: unknown): string {
  if (val === null || val === undefined) return '-'
  if (typeof val === 'object') return JSON.stringify(val)
  return String(val)
}

function handleEdit() {
  emit('edit')
}

function handleStart() {
  emit('start')
}

function handleStop() {
  emit('stop')
}

function handleRefresh() {
  emit('refresh')
}
</script>

<style scoped>
.strategy-detail-panel {
  width: 100%;
}

.panel-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
}

.header-icon {
  color: #409eff;
}

.status-tag {
  margin-left: auto;
}

.panel-content {
  padding: 16px 0;
}

.section {
  margin-bottom: 20px;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #606266;
  margin: 0;
}

.description-text {
  padding: 12px;
  background-color: #f5f7fa;
  border-radius: 4px;
  color: #606266;
  line-height: 1.6;
  white-space: pre-wrap;
}

.tags-container {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tag-item {
  margin: 0;
}

.symbols-container {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.symbol-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.symbol-icon {
  margin-right: 4px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
}

.metric-item {
  padding: 12px;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  background-color: #fafbff;
  transition: all 0.3s;
}

.metric-item:hover {
  transform: translateY(-2px);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

.metric-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.metric-icon {
  color: #409eff;
}

.metric-name {
  font-size: 13px;
  color: #606266;
}

.metric-value {
  font-size: 18px;
  font-weight: 600;
  text-align: right;
}

.metric-value.positive {
  color: #67c23a;
}

.metric-value.negative {
  color: #f56c6c;
}

.metric-value.neutral {
  color: #909399;
}

.metric-change {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  margin-top: 4px;
}

.metric-change.increase {
  color: #67c23a;
}

.metric-change.decrease {
  color: #f56c6c;
}

.metric-change.stable {
  color: #909399;
}

.refresh-label {
  font-size: 12px;
  color: #909399;
  margin-left: 8px;
}

.panel-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
}

.info-descriptions {
  margin-bottom: 20px;
}
</style>
