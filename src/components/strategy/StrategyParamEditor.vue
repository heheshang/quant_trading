<template>
  <div class="param-editor">
    <div v-for="schema in schema" :key="schema.name" class="param-item">
      <el-form-item
        :label="schema.description || schema.name"
        :required="true"
      >
        <!-- Number input -->
        <el-input-number
          v-if="schema.param_type === 'Number'"
          v-model="localValue[schema.name]"
          :min="schema.range?.min ?? -Infinity"
          :max="schema.range?.max ?? Infinity"
          :step="schema.range?.step ?? 1"
          :precision="computePrecision(schema.range?.step)"
          style="width: 100%"
          @change="emitUpdate"
        />
        <!-- String input -->
        <el-input
          v-else-if="schema.param_type === 'String'"
          v-model="localValue[schema.name]"
          :placeholder="schema.description"
          @input="emitUpdate"
        />
        <!-- Select dropdown -->
        <el-select
          v-else-if="isSelectType(schema.param_type)"
          v-model="localValue[schema.name]"
          :placeholder="`请选择${schema.description || schema.name}`"
          style="width: 100%"
          @change="emitUpdate"
        >
          <el-option
            v-for="opt in schema.param_type.Select"
            :key="opt"
            :label="opt"
            :value="opt"
          />
        </el-select>
      </el-form-item>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import type { ParameterSchema } from '@/services/types'

const props = withDefaults(
  defineProps<{
    schema: ParameterSchema[]
    modelValue: Record<string, unknown>
  }>(),
  {
    schema: () => [],
    modelValue: () => ({}),
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: Record<string, unknown>]
}>()

function mergeDefaults(
  modelValue: Record<string, unknown>,
  schema: ParameterSchema[],
): Record<string, unknown> {
  const merged: Record<string, unknown> = { ...modelValue }
  for (const s of schema) {
    if (!(s.name in merged) && s.default !== undefined && s.default !== null) {
      merged[s.name] = s.default
    }
  }
  return merged
}

const localValue = reactive<Record<string, unknown>>(mergeDefaults(props.modelValue, props.schema))

watch(
  () => props.modelValue,
  (val) => {
    Object.assign(localValue, mergeDefaults(val, props.schema))
  },
  { deep: true },
)

function isSelectType(pt: string | { Select: string[] }): pt is { Select: string[] } {
  return typeof pt === 'object' && 'Select' in pt
}

function computePrecision(step?: number): number {
  if (!step) return 0
  const s = step.toString()
  const dot = s.indexOf('.')
  return dot >= 0 ? s.length - dot - 1 : 0
}

function emitUpdate() {
  emit('update:modelValue', { ...localValue })
}
</script>

<style scoped>
.param-editor {
  width: 100%;
}
.param-item {
  margin-bottom: 4px;
}
</style>
