<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    :width="width"
    @update:model-value="$emit('update:visible', $event)"
    @close="$emit('cancel')"
  >
    <div class="confirm-dialog-body">
      <el-icon v-if="typeIcon" :size="24" class="confirm-dialog-icon" :class="`confirm-dialog-icon--${type}`">
        <component :is="typeIcon" />
      </el-icon>
      <span class="confirm-dialog-message">{{ message }}</span>
    </div>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="onCancel">{{ cancelText }}</el-button>
        <el-button :type="buttonType" @click="onConfirm" :loading="confirmLoading">{{ confirmText }}</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, type Component } from 'vue'
import { WarningFilled, InfoFilled, CircleCloseFilled } from '@element-plus/icons-vue'

const props = withDefaults(
  defineProps<{
    visible: boolean
    title?: string
    message?: string
    confirmText?: string
    cancelText?: string
    type?: 'warning' | 'danger' | 'info'
    width?: string
    confirmLoading?: boolean
  }>(),
  {
    visible: false,
    title: '确认操作',
    message: '确定要执行此操作吗？',
    confirmText: '确定',
    cancelText: '取消',
    type: 'warning',
    width: '420px',
    confirmLoading: false,
  },
)

const emit = defineEmits<{
  'update:visible': [value: boolean]
  confirm: []
  cancel: []
}>()

const typeIconMap: Record<string, Component> = {
  warning: WarningFilled,
  danger: CircleCloseFilled,
  info: InfoFilled,
}

const typeIcon = computed(() => typeIconMap[props.type])

const buttonType = computed(() => {
  if (props.type === 'danger') return 'danger'
  if (props.type === 'warning') return 'warning'
  return 'primary'
})

function onConfirm() {
  emit('confirm')
}

function onCancel() {
  emit('cancel')
}
</script>

<style scoped>
.confirm-dialog-body {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 8px 0;
}

.confirm-dialog-icon {
  flex-shrink: 0;
  margin-top: 2px;
}

.confirm-dialog-icon--warning {
  color: #e6a23c;
}

.confirm-dialog-icon--danger {
  color: #f56c6c;
}

.confirm-dialog-icon--info {
  color: #409eff;
}

.confirm-dialog-message {
  font-size: 14px;
  line-height: 1.5;
  color: var(--color-text-regular);
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
