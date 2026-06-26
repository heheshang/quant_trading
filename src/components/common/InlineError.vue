<template>
  <div class="inline-error" :class="`inline-error--${type}`">
    <div class="inline-error-body">
      <el-icon :size="20" class="inline-error-icon">
        <WarningFilled />
      </el-icon>
      <span class="inline-error-text">{{ resolvedMessage }}</span>
    </div>
    <el-button
      v-if="showRetry"
      size="small"
      :type="buttonType"
      plain
      @click="$emit('retry')"
    >
      重试
    </el-button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { WarningFilled } from '@element-plus/icons-vue'

const props = withDefaults(
  defineProps<{
    type?: '4xx' | '5xx' | 'network'
    message?: string
    showRetry?: boolean
  }>(),
  {
    type: '4xx',
    message: undefined,
    showRetry: false,
  },
)

defineEmits<{
  retry: []
}>()

const defaultMessages: Record<string, string> = {
  '4xx': '请求参数有误，请检查后重试',
  '5xx': '服务器繁忙，请稍后重试',
  network: '网络连接异常，请检查网络',
}

const resolvedMessage = computed(() => props.message || defaultMessages[props.type])

const buttonType = computed(() => {
  if (props.type === '4xx') return 'danger'
  return 'warning'
})
</script>

<style scoped>
.inline-error {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-radius: 6px;
  border: 1px solid;
  gap: 12px;
}

.inline-error-body {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.inline-error-icon {
  flex-shrink: 0;
}

.inline-error-text {
  font-size: 14px;
  line-height: 1.4;
}

/* 4xx — red */
.inline-error--4xx {
  background: #fef0f0;
  border-color: #f56c6c;
}
.inline-error--4xx .inline-error-icon {
  color: #f56c6c;
}
.inline-error--4xx .inline-error-text {
  color: #b33a3a;
}

/* 5xx — orange */
.inline-error--5xx {
  background: #fdf6ec;
  border-color: #e6a23c;
}
.inline-error--5xx .inline-error-icon {
  color: #e6a23c;
}
.inline-error--5xx .inline-error-text {
  color: #996b1a;
}

/* network — orange */
.inline-error--network {
  background: #fdf6ec;
  border-color: #e6a23c;
}
.inline-error--network .inline-error-icon {
  color: #e6a23c;
}
.inline-error--network .inline-error-text {
  color: #996b1a;
}
</style>
