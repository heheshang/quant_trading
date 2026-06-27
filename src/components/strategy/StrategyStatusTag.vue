<template>
  <el-tag
    :type="tagType"
    :size="size"
    :effect="effect"
    class="strategy-status-tag"
  >
    <el-icon v-if="showIcon" class="tag-icon">
      <component :is="statusIcon" />
    </el-icon>
    <span class="tag-text">{{ statusText }}</span>
  </el-tag>
</template>

<script setup lang="ts">
import { computed, type Component } from 'vue'
import { CircleCheck, CircleClose, Clock, Warning, Edit } from '@element-plus/icons-vue'

const props = withDefaults(
  defineProps<{
    status: 'active' | 'inactive' | 'pending' | 'error' | 'warning' | 'draft'
    size?: 'default' | 'small' | 'large'
    showIcon?: boolean
    effect?: 'light' | 'dark' | 'plain'
  }>(),
  {
    status: 'draft',
    size: 'default',
    showIcon: true,
    effect: 'light',
  }
)

const statusConfig = {
  active: {
    type: 'success' as const,
    icon: CircleCheck,
    text: '运行中',
  },
  inactive: {
    type: 'info' as const,
    icon: CircleClose,
    text: '已停止',
  },
  pending: {
    type: 'warning' as const,
    icon: Clock,
    text: '待运行',
  },
  error: {
    type: 'danger' as const,
    icon: Warning,
    text: '运行异常',
  },
  warning: {
    type: 'warning' as const,
    icon: Warning,
    text: '预警',
  },
  draft: {
    type: 'info' as const,
    icon: Edit,
    text: '草稿',
  },
}

const statusIcon = computed((): Component => statusConfig[props.status].icon)
const statusText = computed(() => statusConfig[props.status].text)
const tagType = computed(() => statusConfig[props.status].type)
</script>

<style scoped>
.strategy-status-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-weight: 500;
}

.tag-icon {
  margin-right: 4px;
}

.tag-text {
  white-space: nowrap;
}
</style>
