<template>
  <div class="filter-panel">
    <template v-for="filter in filters" :key="filter.key">
      <!-- Select filter -->
      <el-select
        v-if="filter.type === 'select'"
        :model-value="(modelValue as any)[filter.key]"
        :placeholder="filter.placeholder || `请选择${filter.label}`"
        clearable
        class="filter-item"
        @change="onFilterChange(filter.key, $event)"
      >
        <el-option
          v-for="opt in filter.options"
          :key="opt.value"
          :label="opt.label"
          :value="opt.value"
        />
      </el-select>

      <!-- Input filter -->
      <el-input
        v-else-if="filter.type === 'input'"
        :model-value="(modelValue as any)[filter.key]"
        :placeholder="filter.placeholder || `请输入${filter.label}`"
        clearable
        class="filter-item"
        @input="onFilterChange(filter.key, $event)"
      />

      <!-- Date range filter -->
      <el-date-picker
        v-else-if="filter.type === 'dateRange'"
        :model-value="(modelValue as any)[filter.key]"
        type="daterange"
        range-separator="至"
        start-placeholder="开始日期"
        end-placeholder="结束日期"
        value-format="YYYY-MM-DD"
        class="filter-item"
        @change="onFilterChange(filter.key, $event)"
      />
    </template>

    <el-button type="primary" @click="onSearch">筛选</el-button>
    <el-button @click="onReset">重置</el-button>
  </div>
</template>

<script setup lang="ts">
export interface FilterOption {
  key: string
  label: string
  type: 'select' | 'input' | 'dateRange'
  options?: { label: string; value: any }[]
  placeholder?: string
}

const props = defineProps<{
  filters: FilterOption[]
  modelValue: Record<string, any>
}>()

const emit = defineEmits<{
  'update:modelValue': [value: Record<string, any>]
  filter: [value: Record<string, any>]
}>()

function onFilterChange(key: string, value: any) {
  const next = { ...props.modelValue, [key]: value }
  emit('update:modelValue', next)
  emit('filter', next)
}

function onSearch() {
  emit('filter', { ...props.modelValue })
}

function onReset() {
  const empty: Record<string, any> = {}
  for (const key of Object.keys(props.modelValue)) {
    empty[key] = undefined
  }
  emit('update:modelValue', empty)
  emit('filter', {})
}
</script>

<style scoped>
.filter-panel {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}

.filter-item {
  width: 180px;
}
</style>
