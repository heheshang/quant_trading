<template>
  <el-card class="strategy-detail-panel" shadow="hover">
    <template #header>
      <div class="panel-header">
        <el-icon class="header-icon"><Operation /></el-icon>
        <span>策略详情</span>
        <el-tag :type="statusType" size="small" class="status-tag">{{ statusText }}</el-tag>
      </div>
    </template>

    <div class="panel-content">
      <el-descriptions :column="2" border class="info-descriptions">
        <el-descriptions-item label="策略ID">{{ strategyId }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ formatDate(createTime) }}</el-descriptions-item>
        <el-descriptions-item label="最后更新">{{ formatDate(updateTime) }}</el-descriptions-item>
        <el-descriptions-item label="策略类型">{{ strategyType }}</el-descriptions-item>
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
        <h4 class="section-title">指标符号</h4>
        <div class="symbols-container">
          <el-tag
            v-for="symbol in symbols"
            :key="symbol.id"
            size="small"
            :type="getSymbolType(symbol.type)"
            class="symbol-item"
          >
            <el-icon v-if="symbol.icon" class="symbol-icon">
              <component :is="symbol.icon" />
            </el-icon>
            {{ symbol.code }} - {{ symbol.name }}
          </el-tag>
        </div>
      </div>

      <div class="section">
        <div class="section-header">
          <h4 class="section-title">实时指标</h4>
          <el-switch
            v-model="autoRefresh"
            size="small"
            :disabled="!isRunning"
            @change="toggleAutoRefresh"
          />
          <span class="refresh-label">{{ autoRefresh ? '自动刷新' : '手动刷新' }}</span>
        </div>
        <div class="metrics-grid">
          <div v-for="metric in metrics" :key="metric.id" class="metric-item">
            <div class="metric-header">
              <el-icon class="metric-icon">
                <component :is="metric.icon" />
              </el-icon>
              <span class="metric-name">{{ metric.name }}</span>
            </div>
            <div class="metric-value" :class="getMetricClass(metric.value)">
              {{ formatMetricValue(metric.value, metric.unit) }}
            </div>
            <div class="metric-change" :class="getChangeClass(metric.change)">
              <el-icon v-if="metric.change > 0"><Top /></el-icon>
              <el-icon v-else-if="metric.change < 0"><Bottom /></el-icon>
              <span>{{ formatChange(metric.change) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="panel-footer">
        <el-button size="small" @click="handleEdit">编辑策略</el-button>
        <el-button size="small" type="primary" :disabled="!isRunning" @click="handleStart">启动</el-button>
        <el-button size="small" type="danger" :disabled="!isRunning" @click="handleStop">停止</el-button>
        <el-button size="small" @click="handleRefresh">刷新</el-button>
      </div>
    </template>
  </el-card>
</template>

<script setup lang="ts">
import { ref, computed, type Component } from 'vue'
import { Top, Bottom } from '@element-plus/icons-vue'

const props = withDefaults(
  defineProps<{
    strategyId: string
    status: 'active' | 'inactive' | 'pending' | 'error' | 'warning' | 'draft'
    description?: string
    tags?: string[]
    symbols?: {
      id: string
      code: string
      name: string
      type: 'stock' | 'index' | 'etf' | 'crypto'
      icon?: Component
    }[]
    metrics?: {
      id: string
      name: string
      value: number
      unit: string
      change: number
      icon?: Component
    }[]
    createTime?: number
    updateTime?: number
    strategyType?: string
    isRunning?: boolean
  }>(),
  {
    description: '暂无策略描述',
    tags: () => [],
    symbols: () => [],
    metrics: () => [],
    createTime: () => Date.now(),
    updateTime: () => Date.now(),
    strategyType: 'custom',
    isRunning: false,
  }
)

const emit = defineEmits<{
  'edit': []
  'start': []
  'stop': []
  'refresh': []
  'update:metrics': [metrics: typeof props.metrics]
}>()

const autoRefresh = ref(false)

const statusConfig = {
  active: {
    type: 'success' as const,
    text: '运行中',
  },
  inactive: {
    type: 'info' as const,
    text: '已停止',
  },
  pending: {
    type: 'warning' as const,
    text: '待运行',
  },
  error: {
    type: 'danger' as const,
    text: '运行异常',
  },
  warning: {
    type: 'warning' as const,
    text: '预警',
  },
  draft: {
    type: 'info' as const,
    text: '草稿',
  },
}

const statusType = computed(() => statusConfig[props.status].type)
const statusText = computed(() => statusConfig[props.status].text)

function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function getSymbolType(type: string): 'primary' | 'success' | 'warning' | 'info' | 'danger' {
  const typeMap: Record<string, 'primary' | 'success' | 'warning' | 'info' | 'danger'> = {
    stock: 'success',
    index: 'primary',
    etf: 'warning',
    crypto: 'danger',
  }
  return typeMap[type] || 'info'
}

function getMetricClass(value: number): string {
  return value > 0 ? 'positive' : value < 0 ? 'negative' : 'neutral'
}

function getChangeClass(change: number): string {
  return change > 0 ? 'increase' : change < 0 ? 'decrease' : 'stable'
}

function formatMetricValue(value: number, unit: string): string {
  if (Math.abs(value) >= 1000000) {
    return (value / 1000000).toFixed(2) + 'M' + unit
  } else if (Math.abs(value) >= 1000) {
    return (value / 1000).toFixed(2) + 'K' + unit
  }
  return value.toFixed(2) + unit
}

function formatChange(change: number): string {
  const sign = change > 0 ? '+' : ''
  return sign + change.toFixed(2) + '%'
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

function toggleAutoRefresh() {
  if (autoRefresh.value) {
    startAutoRefresh()
  } else {
    stopAutoRefresh()
  }
}

function startAutoRefresh() {
  // 模拟自动刷新逻辑
  console.log('开始自动刷新')
}

function stopAutoRefresh() {
  // 模拟停止自动刷新逻辑
  console.log('停止自动刷新')
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
