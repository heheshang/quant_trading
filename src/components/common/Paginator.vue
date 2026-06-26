<template>
  <div class="paginator-wrapper">
    <el-pagination
      v-bind="$attrs"
      :total="total"
      :current-page="page"
      :page-size="pageSize"
      :page-sizes="pageSizes"
      layout="total, sizes, prev, pager, next, jumper"
      background
      @current-change="onCurrentChange"
      @size-change="onSizeChange"
    />
  </div>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    total: number
    page?: number
    pageSize?: number
    pageSizes?: number[]
  }>(),
  {
    total: 0,
    page: 1,
    pageSize: 20,
    pageSizes: () => [10, 20, 50, 100],
  },
)

const emit = defineEmits<{
  'update:page': [value: number]
  'update:pageSize': [value: number]
}>()

function onCurrentChange(page: number) {
  emit('update:page', page)
}

function onSizeChange(size: number) {
  emit('update:pageSize', size)
  emit('update:page', 1)
}
</script>

<style scoped>
.paginator-wrapper {
  display: flex;
  justify-content: flex-end;
  padding: 16px 0;
}
</style>
