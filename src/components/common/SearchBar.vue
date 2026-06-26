<template>
  <el-input
    :model-value="modelValue"
    :placeholder="placeholder"
    clearable
    @input="onInput"
    @keyup.enter="emitSearch"
    @clear="onClear"
  >
    <template #prefix>
      <el-icon><Search /></el-icon>
    </template>
  </el-input>
</template>

<script setup lang="ts">
import { watch, ref } from 'vue'
import { Search } from '@element-plus/icons-vue'

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
  }>(),
  {
    placeholder: '搜索...',
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  search: [value: string]
}>()

const debounceTimer = ref<ReturnType<typeof setTimeout> | null>(null)

function onInput(value: string) {
  emit('update:modelValue', value)
  if (debounceTimer.value) clearTimeout(debounceTimer.value)
  debounceTimer.value = setTimeout(() => {
    emit('search', value)
  }, 300)
}

function emitSearch() {
  if (debounceTimer.value) clearTimeout(debounceTimer.value)
  emit('search', props.modelValue)
}

function onClear() {
  emit('update:modelValue', '')
  emit('search', '')
}

// Cleanup on unmount
watch(
  () => props.modelValue,
  () => {},
)
</script>
