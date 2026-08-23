<template>
  <el-card class="stats-card" :class="{ clickable: !!to }" :body-style="{ padding: '20px' }" @click="$emit('click')">
    <el-skeleton v-if="loading" :rows="2" animated />
    <div v-else class="stat-item">
      <div v-if="icon" class="stat-icon" :style="{ background: iconBg }">
        <el-icon><component :is="icon" /></el-icon>
      </div>
      <div class="stat-info">
        <div class="stat-label">{{ title }}</div>
        <div class="stat-value" :class="valueClass">
          <template v-if="trend !== undefined && trend > 0">+</template>
          {{ formattedValue }}
        </div>
      </div>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { computed, type Component } from 'vue'

const props = withDefaults(
  defineProps<{
    title: string
    value: string | number
    icon?: string | Component
    iconBg?: string
    trend?: number
    loading?: boolean
    format?: 'currency' | 'number' | 'percentage' | 'raw'
    to?: string
  }>(),
  {
    icon: undefined,
    iconBg: 'var(--color-primary)',
    trend: undefined,
    loading: false,
    format: 'raw',
  },
)

defineEmits<{ click: [] }>()

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

const valueClass = computed(() => {
  if (props.trend === undefined) return undefined
  return { positive: props.trend > 0, negative: props.trend < 0 }
})
</script>

<style scoped>
.stat-item {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.stat-icon {
  width: 60px;
  height: 60px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: #fff;
  flex-shrink: 0;
}

.stat-info {
  flex: 1;
  min-width: 0;
}

.stat-label {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  margin-bottom: 6px;
}

.stat-value {
  font-size: var(--font-size-xl);
  font-weight: 700;
  color: var(--color-text-primary);
  line-height: 1.2;
}

.stats-card.clickable {
  cursor: pointer;
  transition: transform var(--transition-fast), box-shadow var(--transition-fast);
}
.stats-card.clickable:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-lg);
}

.stat-value.positive {
  color: var(--color-success);
}

.stat-value.negative {
  color: var(--color-danger);
}
</style>
