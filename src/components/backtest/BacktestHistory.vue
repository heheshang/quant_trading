<template>
  <el-card class="history-card">
    <template #header>
      <div class="card-header">
        <span>回测历史记录</span>
        <div class="card-header-controls">
          <el-button @click="toggleCompare" size="small" :type="compareMode ? 'primary' : 'default'">
            {{ compareMode ? '取消对比' : '对比' }}
          </el-button>
          <el-button @click="exportCSV" size="small">导出CSV</el-button>
          <el-button @click="$emit('refresh')" :loading="historyLoading" size="small">刷新</el-button>
        </div>
      </div>
    </template>

    <el-table
      v-if="historyRecords.length > 0"
      ref="tableRef"
      :data="historyRecords"
      style="width: 100%"
      v-loading="historyLoading"
      @selection-change="onSelectionChange"
    >
      <el-table-column v-if="compareMode" type="selection" width="50" />
      <el-table-column prop="strategy_name" label="策略名称" width="150">
        <template #default="scope"><span>{{ scope.row.strategy_name || '-' }}</span></template>
      </el-table-column>
      <el-table-column prop="start_date" label="开始日期" width="120">
        <template #default="scope">{{ formatDate(scope.row.start_date) }}</template>
      </el-table-column>
      <el-table-column prop="end_date" label="结束日期" width="120">
        <template #default="scope">{{ formatDate(scope.row.end_date) }}</template>
      </el-table-column>
      <el-table-column prop="total_return" label="总收益率" width="100">
        <template #default="scope">
          <span :class="signClass(scope.row.total_return)">{{ formatPercentage(scope.row.total_return) }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="sharpe_ratio" label="夏普比率" width="100">
        <template #default="scope">{{ formatNumber(scope.row.sharpe_ratio) }}</template>
      </el-table-column>
      <el-table-column prop="max_drawdown" label="最大回撤" width="100">
        <template #default="scope"><span class="negative">{{ formatPercentage(scope.row.max_drawdown) }}</span></template>
      </el-table-column>
      <el-table-column prop="total_trades" label="交易数" width="80" />
      <el-table-column prop="win_rate" label="胜率" width="80">
        <template #default="scope">{{ formatPercentage(scope.row.win_rate) }}</template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="160">
        <template #default="scope">{{ formatDate(scope.row.created_at) }}</template>
      </el-table-column>
      <el-table-column label="操作" width="160" fixed="right">
        <template #default="scope">
          <el-button size="small" @click="$emit('viewDetail', scope.row.id)">详情</el-button>
          <el-button size="small" type="danger" @click="promptDeleteRecord(scope.row.id)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>
    <EmptyState v-else-if="!historyLoading" title="暂无回测记录" description="请先运行回测来生成记录" />
  </el-card>

  <el-dialog v-model="compareDialogVisible" title="结果对比" width="900px">
    <el-row :gutter="20" v-if="compareResults.length === 2">
      <el-col :span="12" v-for="(r, i) in compareResults" :key="i">
        <el-card>
          <template #header><span>{{ r.strategy_name || `结果 ${i + 1}` }}</span></template>
          <div class="compare-stats">
            <div class="compare-row">
              <span class="cl">总收益率</span>
              <span :class="signClass(r.total_return)">{{ formatPercentage(r.total_return) }}</span>
            </div>
            <div class="compare-row">
              <span class="cl">年化收益率</span>
              <span :class="signClass(r.annual_return)">{{ formatPercentage(r.annual_return) }}</span>
            </div>
            <div class="compare-row">
              <span class="cl">夏普比率</span><span>{{ formatNumber(r.sharpe_ratio) }}</span>
            </div>
            <div class="compare-row">
              <span class="cl">最大回撤</span><span class="negative">{{ formatPercentage(r.max_drawdown) }}</span>
            </div>
            <div class="compare-row">
              <span class="cl">胜率</span><span>{{ formatPercentage(r.win_rate) }}</span>
            </div>
            <div class="compare-row">
              <span class="cl">总交易数</span><span>{{ r.total_trades }}</span>
            </div>
            <div class="compare-row">
              <span class="cl">盈亏比</span><span>{{ formatNumber(r.profit_loss_ratio) }}</span>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
    <p v-else style="text-align:center;color:var(--color-text-secondary);">请选择两条记录进行对比</p>
    <template #footer>
      <el-button @click="compareDialogVisible = false">关闭</el-button>
    </template>
  </el-dialog>

  <ConfirmDialog
    v-model:visible="deleteDialogVisible"
    title="确认删除"
    message="确定要删除这条回测记录吗？此操作不可撤销。"
    type="danger"
    confirm-text="删除"
    @confirm="confirmDelete"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { BacktestResultSummaryRow } from '@/services/types'
import { useFormatting } from '@/composables/useFormatting'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import EmptyState from '@/components/common/EmptyState.vue'

export interface HistoryRow extends BacktestResultSummaryRow {
  annual_return?: number
  profit_loss_ratio?: number
}

const props = defineProps<{
  historyRecords: HistoryRow[]
  historyLoading: boolean
}>()

const emit = defineEmits<{
  viewDetail: [id: number]
  refresh: []
  deleteRecord: [id: number]
}>()

const { formatPercentage, formatNumber, formatDate } = useFormatting()

// Compare mode
const compareMode = ref(false)
const compareDialogVisible = ref(false)
const compareResults = ref<HistoryRow[]>([])
const selectedIds = ref<number[]>([])

function toggleCompare() {
  compareMode.value = !compareMode.value
  if (!compareMode.value) {
    selectedIds.value = []
    compareResults.value = []
  }
}

function onSelectionChange(rows: HistoryRow[]) {
  selectedIds.value = rows.map((r) => r.id)
  if (compareMode.value && rows.length === 2) {
    compareResults.value = rows
    compareDialogVisible.value = true
  }
}

// Delete confirmation
const deleteDialogVisible = ref(false)
const recordToDelete = ref<number | null>(null)

function promptDeleteRecord(id: number) {
  recordToDelete.value = id
  deleteDialogVisible.value = true
}

function confirmDelete() {
  if (recordToDelete.value !== null) {
    emit('deleteRecord', recordToDelete.value)
  }
  deleteDialogVisible.value = false
  recordToDelete.value = null
}

// CSV export
function exportCSV() {
  const headers = ['策略名称', '开始日期', '结束日期', '总收益率', '夏普比率', '最大回撤', '交易数', '胜率', '创建时间']
  const data = props.historyRecords.map((r) => [
    r.strategy_name,
    r.start_date,
    r.end_date,
    formatPercentage(r.total_return),
    formatNumber(r.sharpe_ratio),
    formatPercentage(r.max_drawdown),
    r.total_trades,
    formatPercentage(r.win_rate),
    r.created_at,
  ])
  const csv = [headers.join(','), ...data.map((row) => row.join(','))].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `backtest_history_${new Date().toISOString().slice(0, 10)}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

function signClass(value: number | null | undefined): string {
  if (!value) return ''
  if (value > 0) return 'positive'
  if (value < 0) return 'negative'
  return ''
}
</script>

<style scoped>
.history-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.compare-stats {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.compare-row {
  display: flex;
  justify-content: space-between;
  font-size: 14px;
  padding: 4px 0;
  border-bottom: 1px solid #f0f0f0;
}

.compare-row .cl {
  color: var(--color-text-regular);
}

:deep(.positive) {
  color: #67c23a;
}

:deep(.negative) {
  color: #f56c6c;
}
</style>
