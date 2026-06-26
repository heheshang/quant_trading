<template>
  <div class="empty-state">
    <div class="empty-state-icon">
      <el-icon :size="48">
        <component :is="resolvedIcon" />
      </el-icon>
    </div>
    <h3 class="empty-state-title">{{ title }}</h3>
    <p class="empty-state-desc">{{ description }}</p>
    <div v-if="$slots.default" class="empty-state-action">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, type Component } from 'vue'
import { Search, WarningFilled, FolderDelete } from '@element-plus/icons-vue'

const props = withDefaults(
  defineProps<{
    icon?: string | Component
    title?: string
    description?: string
    type?: 'empty' | 'search' | 'error'
  }>(),
  {
    icon: undefined,
    title: '暂无数据',
    description: '当前没有可显示的内容',
    type: 'empty',
  },
)

const typeIcons: Record<string, Component> = {
  empty: FolderDelete,
  search: Search,
  error: WarningFilled,
}

const resolvedIcon = computed(() => props.icon || typeIcons[props.type] || FolderDelete)
</script>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 60px 20px;
  text-align: center;
}

.empty-state-icon {
  margin-bottom: 16px;
  color: #d9d9d9;
}

.empty-state-title {
  margin: 0 0 8px;
  font-size: 18px;
  font-weight: 700;
  color: #303133;
}

.empty-state-desc {
  margin: 0 0 24px;
  font-size: 14px;
  color: #909399;
  line-height: 1.5;
}

.empty-state-action {
  display: flex;
  gap: 12px;
  justify-content: center;
}
</style>
