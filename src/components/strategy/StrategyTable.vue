<template>
  <div class="strategy-table">
    <div class="table-toolbar">
      <SearchBar
        class="search-bar"
        :model-value="searchQuery"
        placeholder="搜索策略名称"
        @update:model-value="$emit('update:searchQuery', $event)"
        @search="onSearch"
      />
      <FilterPanel
        :model-value="activeFilters"
        :filters="filterOptions"
        @update:model-value="$emit('update:activeFilters', $event)"
        @filter="onFilterChange"
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
      <el-table-column prop="strategy_name" label="策略名称" min-width="190" show-overflow-tooltip>
        <template #default="scope"><span class="strategy-name">{{ scope.row.strategy_name }}</span></template>
      </el-table-column>
      <el-table-column prop="strategy_type" label="策略类型" width="120">
        <template #default="scope">
          {{ formatStrategyType(scope.row.strategy_type) }}
        </template>
      </el-table-column>
      <el-table-column label="状态" width="100">
        <template #default="scope">
          <StrategyStatusTag :status="scope.row.status" size="small" />
        </template>
      </el-table-column>
      <el-table-column label="启用" width="80">
        <template #default="scope">
          <el-switch
            :model-value="scope.row.enabled"
            :aria-label="`启用策略 ${scope.row.strategy_name}`"
            @update:model-value="(v: boolean) => emit('toggle', scope.row.strategy_id, v)"
          />
        </template>
      </el-table-column>
      <el-table-column prop="max_position" label="最大持仓" width="120" align="right">
        <template #default="scope">
          ¥{{ formatCurrency(scope.row.max_position) }}
        </template>
      </el-table-column>
      <el-table-column prop="max_daily_loss" label="最大日亏损" width="120" align="right">
        <template #default="scope">
          ¥{{ formatCurrency(scope.row.max_daily_loss) }}
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="180">
        <template #default="scope">
          {{ formatDate(scope.row.created_at) }}
        </template>
      </el-table-column>
      <el-table-column label="操作" width="136" fixed="right">
        <template #default="scope">
          <div class="action-cell">
            <el-button size="small" text type="primary" @click="emit('detail', scope.row)">详情</el-button>
            <el-dropdown trigger="click">
              <el-button size="small" text class="action-menu-btn" :aria-label="`策略操作菜单 ${scope.row.strategy_name}`">更多<el-icon><ArrowDown /></el-icon></el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item @click="emit('edit', scope.row)">编辑</el-dropdown-item>
                  <el-dropdown-item @click="emit('backtest', scope.row.strategy_id)">回测</el-dropdown-item>
                  <el-dropdown-item @click="emit('optimize', scope.row)">参数优化</el-dropdown-item>
                  <el-dropdown-item divided @click="emit('lifecycle', 'deploy', scope.row)">部署</el-dropdown-item>
                  <el-dropdown-item @click="emit('lifecycle', 'start', scope.row)">启动</el-dropdown-item>
                  <el-dropdown-item @click="emit('lifecycle', 'stop', scope.row)">停止</el-dropdown-item>
                  <el-dropdown-item @click="emit('lifecycle', 'pause', scope.row)">暂停</el-dropdown-item>
                  <el-dropdown-item @click="emit('lifecycle', 'resume', scope.row)">恢复</el-dropdown-item>
                  <el-dropdown-item @click="emit('lifecycle', 'archive', scope.row)">归档</el-dropdown-item>
                  <el-dropdown-item divided class="danger-action" @click="emit('delete', scope.row.strategy_id)">删除</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </template>
      </el-table-column>
    </el-table>

    <div v-if="total > 0" class="table-footer">
      <Paginator
        :total="total"
        :page-size="pageSize"
        :page="currentPage"
        @update:page="onPageChange"
        @update:page-size="onPageSizeChange"
      />
    </div>
    <EmptyState v-else-if="!loading" title="暂无策略" description="点击「新建策略」按钮创建第一个策略" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
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
  optimize: [strategy: StrategyParams]
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
.strategy-table { width: 100%; min-width: 0; }
.table-toolbar { display: flex; flex-wrap: wrap; align-items: center; margin-bottom: 24px; gap: 12px; }
.table-toolbar :deep(.search-bar) { width: 260px; max-width: 100%; }
.table-toolbar :deep(.filter-panel) { gap: 8px; }
.table-toolbar :deep(.filter-panel .el-button) { margin-left: 0; }
.strategy-name { font-weight: 600; color: var(--color-text-primary); }
.action-menu-btn .el-icon { margin-left: 4px; }
.strategy-table :deep(.el-table) { border-radius: 8px; }
.batch-actions { display: flex; flex-wrap: wrap; gap: var(--space-xs); margin-left: auto; }
.table-footer { margin-top: 16px; border-top: 1px solid var(--color-border-light); padding-top: 8px; }
.action-cell {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
}

.action-cell :deep(.el-button) { margin-left: 0; }
.danger-action { color: var(--color-danger); }

@media (max-width: 768px) {
  .table-toolbar > * { max-width: 100%; }
  .table-toolbar :deep(.search-bar) { width: 100%; }
  .table-toolbar :deep(.filter-panel) { width: 100%; display: grid; grid-template-columns: minmax(0, 1fr) auto auto; }
  .table-toolbar :deep(.filter-item) { width: 100%; min-width: 0; }
  .table-toolbar :deep(.filter-panel .el-button) { padding-inline: 10px; }
  .batch-actions { margin-left: 0; }
  .action-cell { gap: var(--space-xxs); }
  .action-cell :deep(.el-button) { padding: 8px; }
}
</style>
