<template>
  <el-card class="metric-card" :body-style="{ padding: '16px' }">
    <div class="metric-header">
      <div v-if="icon" class="metric-icon" :style="{ background: iconBg }">
        <el-icon><component :is="icon" /></el-icon>
      </div>
      <div class="metric-info">
        <div class="metric-label">{{ title }}</div>
        <div class="metric-value">
          {{ formattedValue }}
          <span v-if="unit" class="metric-unit">{{ unit }}</span>
        </div>
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    title: string
    value: string | number
    unit?: string
    icon?: string
    iconBg?: string
    format?: 'currency' | 'number' | 'percentage' | 'raw'
  }>(),
  {
    unit: undefined,
    icon: undefined,
    iconBg: '#409eff',
    format: 'raw',
  },
)

const formattedValue = computed(() => {
  const v = props.value
  switch (props.format) {
    case 'currency':
      return '¥' + Number(v).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
    case 'number':
      return Number(v).toLocaleString('zh-CN')
    case 'percentage':
      return (Number(v) * 100).toFixed(2) + '%'
    default:
      return String(v)
  }
})
</script>

<style scoped>
.metric-card {
  margin-bottom: 0;
}

.metric-header {
  display: flex;
  align-items: center;
  gap: 16px;
}

.metric-icon {
  width: 48px;
  height: 48px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  color: #fff;
  flex-shrink: 0;
}

.metric-info {
  flex: 1;
  min-width: 0;
}

.metric-label {
  font-size: 14px;
  color: #999;
  margin-bottom: 4px;
}

.metric-value {
  font-size: 20px;
  font-weight: bold;
  color: #333;
}

.metric-unit {
  font-size: 14px;
  font-weight: normal;
  color: #999;
  margin-left: 4px;
}
</style>
