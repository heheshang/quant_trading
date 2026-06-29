<template>
  <div class="strategy-table">
    <div class="table-toolbar">
      <SearchBar
        :model-value="searchQuery"
        placeholder="搜索策略名称"
        @update:model-value="$emit('update:searchQuery', $event)"
        @search="onSearch"
      />
      <FilterPanel
        :model-value="activeFilters"
        :filters="filterOptions"
        @update:model-value="$emit('update:activeFilters', $event)"
        @change="onFilterChange"
      />
      <div class="batch-actions" v-if="selected.length > 0">
        <el-button size="small" @click="emitBatch('start')">批量启动</el-button>
        <el-button size="small" @click="emitBatch('stop')">批量停止</el-button>
        <el-button size="small" type="danger" @click="emitBatch('delete')">批量删除</el-button>
      </div>
    </div>

    <el-table
      v-if="paginated.length > 0"
      :data="paginated"
      style="width: 100%"
      v-loading="loading"
      @selection-change="onSelectionChange"
    >
      <el-table-column type="selection" width="50" />
      <el-table-column prop="strategy_name" label="策略名称" width="180" />
      <el-table-column prop="strategy_type" label="策略类型" width="120">
        <template #default="scope">
          {{ formatStrategyType(scope.row.strategy_type) }}
        </template>
      </el-table-column>
      <el-table-column label="状态" width="120">
        <template #default="scope">
          <StrategyStatusTag :status="scope.row.status" size="small" />
        </template>
      </el-table-column>
      <el-table-column label="启用" width="80">
        <template #default="scope">
          <el-switch
            :model-value="scope.row.enabled"
            @update:model-value="(v: boolean) => emit('toggle', scope.row.strategy_id, v)"
            active-text="是"
            inactive-text="否"
          />
        </template>
      </el-table-column>
      <el-table-column prop="max_position" label="最大持仓" width="120">
        <template #default="scope">
          ¥{{ formatCurrency(scope.row.max_position) }}
        </template>
      </el-table-column>
      <el-table-column prop="max_daily_loss" label="最大日亏损" width="120">
        <template #default="scope">
          ¥{{ formatCurrency(scope.row.max_daily_loss) }}
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="180">
        <template #default="scope">
          {{ formatDate(scope.row.created_at) }}
        </template>
      </el-table-column>
      <el-table-column label="操作" width="280">
        <template #default="scope">
          <el-dropdown trigger="click" @command="(cmd: string) => emit('lifecycle', cmd, scope.row)">
            <el-button size="small" type="primary">
              生命周期 <el-icon class="el-icon--right"><ArrowDown /></el-icon>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="deploy">部署</el-dropdown-item>
                <el-dropdown-item command="start">启动</el-dropdown-item>
                <el-dropdown-item command="stop">停止</el-dropdown-item>
                <el-dropdown-item command="pause">暂停</el-dropdown-item>
                <el-dropdown-item command="resume">恢复</el-dropdown-item>
                <el-dropdown-item command="archive" divided>归档</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <el-button size="small" @click="emit('detail', scope.row)">详情</el-button>
          <el-button size="small" @click="emit('edit', scope.row)">编辑</el-button>
          <el-button size="small" type="primary" @click="emit('backtest', scope.row.strategy_id)">回测</el-button>
          <el-button size="small" type="danger" @click="emit('delete', scope.row.strategy_id)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div v-if="total > 0" class="table-footer">
      <Paginator
        :total="total"
        :page-size="pageSize"
        :current-page="currentPage"
        @update:current-page="onPageChange"
        @update:page-size="onPageSizeChange"
      />
    </div>
    <EmptyState v-else-if="!loading" title="暂无策略" description="点击「新建策略」按钮创建第一个策略" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { ArrowDown } from '@element-plus/icons-vue'
import type { StrategyParams } from '@/services/types'
import { useFormatting } from '@/composables/useFormatting'
import StrategyStatusTag from '@/components/strategy/StrategyStatusTag.vue'
import SearchBar from '@/components/common/SearchBar.vue'
import FilterPanel from '@/components/common/FilterPanel.vue'
import Paginator from '@/components/common/Paginator.vue'
import EmptyState from '@/components/common/EmptyState.vue'
import type { FilterOption } from '@/components/common/FilterPanel.vue'

const { formatCurrency, formatDate, formatStrategyType } = useFormatting()

const props = withDefaults(
  defineProps<{
    strategies: StrategyParams[]
    strategyTypes: { type_name: string; display_name?: string }[]
    loading: boolean
    searchQuery: string
    activeFilters: Record<string, any>
    pageSize: number
    currentPage: number
    selected: StrategyParams[]
  }>(),
  {
    strategies: () => [],
    strategyTypes: () => [],
    activeFilters: () => ({}),
    selected: () => [],
  },
)

const emit = defineEmits<{
  'update:searchQuery': [value: string]
  'update:activeFilters': [value: Record<string, any>]
  'update:pageSize': [value: number]
  'update:currentPage': [value: number]
  'update:selected': [value: StrategyParams[]]
  search: [value: string]
  toggle: [strategyId: string, enabled: boolean]
  detail: [strategy: StrategyParams]
  edit: [strategy: StrategyParams]
  delete: [strategyId: string]
  backtest: [strategyId: string]
  lifecycle: [action: string, strategy: StrategyParams]
  'batch-start': [strategies: StrategyParams[]]
  'batch-stop': [strategies: StrategyParams[]]
  'batch-delete': [strategies: StrategyParams[]]
}>()

const filterOptions = computed<FilterOption[]>(() => [
  {
    key: 'strategy_type', label: '策略类型', type: 'select',
    options: props.strategyTypes.map(t => ({ label: t.display_name || t.type_name, value: t.type_name })),
  },
])

const filteredStrategies = computed(() => {
  let list = props.strategies
  if (props.searchQuery) {
    const q = props.searchQuery.toLowerCase()
    list = list.filter((s) => s.strategy_name?.toLowerCase().includes(q))
  }
  if (props.activeFilters.strategy_type) {
    list = list.filter((s) => s.strategy_type === props.activeFilters.strategy_type)
  }
  return list
})

const paginated = computed(() => {
  const start = (props.currentPage - 1) * props.pageSize
  return filteredStrategies.value.slice(start, start + props.pageSize)
})

const total = computed(() => filteredStrategies.value.length)

function onSearch() { emit('update:currentPage', 1); emit('search', props.searchQuery) }
function onFilterChange() { emit('update:currentPage', 1) }
function onPageChange(page: number) { emit('update:currentPage', page) }
function onPageSizeChange(size: number) { emit('update:pageSize', size) }
function onSelectionChange(rows: StrategyParams[]) { emit('update:selected', rows) }

function emitBatch(action: 'start' | 'stop' | 'delete') {
  if (props.selected.length === 0) return
  if (action === 'start') emit('batch-start', props.selected)
  else if (action === 'stop') emit('batch-stop', props.selected)
  else emit('batch-delete', props.selected)
}
</script>

<style scoped>
.strategy-table { width: 100%; }
.table-toolbar { display: flex; flex-wrap: wrap; align-items: flex-start; margin-bottom: 12px; gap: 12px; }
.batch-actions { display: flex; gap: 8px; flex-shrink: 0; margin-left: auto; }
.table-footer { margin-top: 16px; display: flex; justify-content: flex-end; }
</style>
