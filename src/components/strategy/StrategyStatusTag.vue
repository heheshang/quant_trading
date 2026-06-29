<template>
  <el-tag
    :type="display.type"
    :size="size"
    :effect="effect"
    class="strategy-status-tag"
  >
    <el-icon v-if="showIcon" class="tag-icon">
      <component :is="display.icon" />
    </el-icon>
    <span class="tag-text">{{ display.label }}</span>
  </el-tag>
</template>

<script setup lang="ts">
import { toRef } from 'vue'
import type { StrategyStatus } from '@/services/types'
import { useStrategyStatus } from '@/composables/useStrategyStatus'

const props = withDefaults(
  defineProps<{
    status: StrategyStatus
    size?: 'default' | 'small' | 'large'
    showIcon?: boolean
    effect?: 'light' | 'dark' | 'plain'
  }>(),
  {
    size: 'default',
    showIcon: true,
    effect: 'light',
  }
)

const statusRef = toRef(props, 'status')
const display = useStrategyStatus(statusRef)
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
