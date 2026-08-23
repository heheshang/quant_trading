<template>
  <el-dialog v-model="dialogVisible" title="参数优化" width="760px" :close-on-click-modal="false">
    <el-form label-width="120px">
      <el-form-item label="策略">
        <el-input :model-value="strategyName" disabled />
      </el-form-item>
      <el-form-item label="参数网格 (JSON)">
        <el-input
          v-model="paramGridText"
          type="textarea"
          :rows="4"
          placeholder='{"rsi_period": [14, 7], "entry_threshold": [30, 25]}'
        />
      </el-form-item>
      <el-row :gutter="16">
        <el-col :span="8">
          <el-form-item label="优化指标">
            <el-select v-model="metric" style="width: 100%">
              <el-option label="夏普比率" value="sharpe_ratio" />
              <el-option label="年化收益" value="annual_return" />
              <el-option label="最大回撤" value="max_drawdown" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="算法">
            <el-select v-model="algorithm" style="width: 100%" disabled>
              <el-option label="网格搜索 (GridSearch)" value="grid_search" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="Top-N">
            <el-input-number v-model="topN" :min="1" :max="50" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="初始资金">
        <el-input-number v-model="initialCapital" :min="1000" :step="1000" style="width: 200px" />
      </el-form-item>
      <el-form-item label="日期范围">
        <el-date-picker
          v-model="dateRange"
          type="daterange"
          value-format="YYYY-MM-DD"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          style="width: 100%"
        />
      </el-form-item>
    </el-form>

    <div v-if="result" class="optimizer-result">
      <el-alert
        type="success"
        :closable="false"
        show-icon
        :title="`共 ${result.total_combinations} 组组合，返回 Top ${result.combinations_returned}`"
        style="margin-bottom: 12px"
      />
      <template v-if="result.best">
        <p style="font-weight:bold;margin:8px 0;">最优参数</p>
        <el-descriptions :column="2" border size="small" style="margin-bottom: 14px">
          <el-descriptions-item label="分组">{{ result.best.label }}</el-descriptions-item>
          <el-descriptions-item label="参数">
            <code>{{ JSON.stringify(result.best.params) }}</code>
          </el-descriptions-item>
        </el-descriptions>
      </template>

      <p style="font-weight:bold;margin:8px 0;">Top 组合</p>
      <el-table :data="result.combinations" border size="small" style="width: 100%">
        <el-table-column prop="label" label="分组" width="140" />
        <el-table-column label="参数" min-width="180">
          <template #default="scope">
            <code>{{ JSON.stringify(scope.row.params) }}</code>
          </template>
        </el-table-column>
        <el-table-column label="总收益" width="100">
          <template #default="scope">
            {{ formatPct(scope.row.result?.total_return) }}
          </template>
        </el-table-column>
        <el-table-column label="年化" width="100">
          <template #default="scope">
            {{ formatPct(scope.row.result?.annual_return) }}
          </template>
        </el-table-column>
        <el-table-column label="夏普" width="90">
          <template #default="scope">
            {{ scope.row.result?.sharpe_ratio?.toFixed(2) ?? '—' }}
          </template>
        </el-table-column>
        <el-table-column label="最大回撤" width="110">
          <template #default="scope">
            {{ formatPct(scope.row.result?.max_drawdown) }}
          </template>
        </el-table-column>
        <el-table-column label="胜率" width="90">
          <template #default="scope">
            {{ formatPct(scope.row.result?.win_rate) }}
          </template>
        </el-table-column>
        <el-table-column label="交易数" width="90">
          <template #default="scope">
            {{ scope.row.result?.total_trades ?? '—' }}
          </template>
        </el-table-column>
      </el-table>
    </div>

    <template #footer>
      <el-button @click="dialogVisible = false">关闭</el-button>
      <el-button type="primary" @click="runOptimization" :loading="running">运行优化</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { optimizeStrategy, type OptimizationResult } from '@/services/optimizer'

const props = defineProps<{
  visible: boolean
  strategyId?: string
  strategyName?: string
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const dialogVisible = computed({
  get: () => props.visible,
  set: (val: boolean) => emit('update:visible', val),
})

const paramGridText = ref('')
const metric = ref('sharpe_ratio')
const algorithm = ref('grid_search')
const topN = ref(5)
const initialCapital = ref(10000)
const dateRange = ref<[string, string] | null>(null)
const running = ref(false)
const result = ref<OptimizationResult | null>(null)

function formatPct(value: number | undefined | null): string {
  if (value === undefined || value === null || Number.isNaN(value)) return '—'
  return `${(value * 100).toFixed(2)}%`
}

async function runOptimization() {
  if (!props.strategyId) {
    ElMessage.warning('请先选择策略')
    return
  }
  let paramGrid: unknown
  try {
    paramGrid = paramGridText.value.trim() ? JSON.parse(paramGridText.value) : {}
  } catch {
    ElMessage.error('参数网格不是合法的 JSON')
    return
  }
  running.value = true
  try {
    result.value = await optimizeStrategy({
      strategyId: props.strategyId,
      paramGrid,
      metric: metric.value,
      algorithm: algorithm.value,
      topN: topN.value,
      initialCapital: initialCapital.value,
      startDate: dateRange.value?.[0] ?? undefined,
      endDate: dateRange.value?.[1] ?? undefined,
    })
  } catch (error) {
    console.error('Failed to optimize strategy:', error)
    ElMessage.error('参数优化失败')
  } finally {
    running.value = false
  }
}
</script>

<style scoped>
.optimizer-result {
  margin-top: 8px;
}
</style>
