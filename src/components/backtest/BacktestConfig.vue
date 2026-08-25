<template>
  <el-card class="backtest-config-card">
    <template #header>
      <div class="card-header">
        <span>回测配置</span>
      </div>
    </template>

    <el-form :model="formConfig" :rules="rules" ref="formRef" label-width="120px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="选择策略" prop="strategyId">
            <el-select v-model="formConfig.strategyId" placeholder="请选择策略" @change="onStrategyChange">
              <el-option
                v-for="strategy in strategies"
                :key="strategy.strategy_id"
                :label="strategy.strategy_name"
                :value="strategy.strategy_id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="策略名称">
            <el-input v-model="formConfig.strategyName" readonly />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="开始日期" prop="startDate">
            <el-date-picker
              v-model="formConfig.startDate"
              type="date"
              placeholder="选择开始日期"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="结束日期" prop="endDate">
            <el-date-picker
              v-model="formConfig.endDate"
              type="date"
              placeholder="选择结束日期"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="初始资金" prop="initialCapital">
            <el-input-number v-model="formConfig.initialCapital" :min="10000" :step="100000" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="手续费率" prop="commissionRate">
            <el-input-number v-model="formConfig.commissionRate" :min="0" :max="0.1" :step="0.001" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="滑点" prop="slippage">
            <el-input-number v-model="formConfig.slippage" :min="0" :max="0.1" :step="0.001" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="标的代码" prop="symbols">
            <el-select
              v-model="selectedSymbols"
              multiple
              filterable
              placeholder="选择标的代码"
              style="width: 100%"
            >
              <el-option v-for="s in symbolList" :key="s" :label="s" :value="s" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="K线周期" prop="timeframe">
            <el-select v-model="formConfig.timeframe" style="width: 100%">
              <el-option label="1小时 (1H)" value="1H" />
              <el-option label="4小时 (4H)" value="4H" />
              <el-option label="日线 (1D)" value="1D" />
              <el-option label="周线 (1W)" value="1W" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item>
            <div class="form-actions">
              <el-button type="primary" @click="handleRun" :loading="running">开始回测</el-button>
              <el-button @click="handleSaveTemplate">保存模板</el-button>
              <el-dropdown @command="handleLoadTemplate" v-if="templates.length > 0">
                <el-button>
                  加载模板<el-icon class="el-icon--right"><ArrowDown /></el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item v-for="(tpl, i) in templates" :key="i" :command="i">
                      {{ tpl.name }}
                    </el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
              <el-button @click="handleReset">重置</el-button>
            </div>
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, ref, computed, onMounted } from 'vue'
import type { FormInstance } from 'element-plus'
import { ArrowDown } from '@element-plus/icons-vue'
import type { StrategyParams } from '@/services/types'
import { ElMessage } from 'element-plus'
import { getSymbols } from '@/services/market'

export interface BacktestConfigData {
  strategyId: string
  strategyName: string
  startDate: string
  endDate: string
  initialCapital: number
  commissionRate: number
  slippage: number
  symbols: string
  timeframe: string
}

export interface ConfigTemplate {
  name: string
  config: BacktestConfigData
}

const props = defineProps<{
  strategies: StrategyParams[]
  running: boolean
  templates: ConfigTemplate[]
}>()

const emit = defineEmits<{
  run: [config: BacktestConfigData]
  saveTemplate: [config: BacktestConfigData]
  loadTemplate: [index: number]
  reset: []
}>()

function defaultConfig(): BacktestConfigData {
  const today = new Date()
  const oneMonthAgo = new Date()
  oneMonthAgo.setMonth(oneMonthAgo.getMonth() - 1)
  return {
    strategyId: '',
    strategyName: '',
    startDate: oneMonthAgo.toISOString().split('T')[0],
    endDate: today.toISOString().split('T')[0],
    initialCapital: 1000000,
    commissionRate: 0.001,
    slippage: 0.0005,
    symbols: 'BTC-USDT',
    timeframe: '1H',
  }
}

const formRef = ref<FormInstance | undefined>()
const formConfig = reactive<BacktestConfigData>(defaultConfig())
/** 标的代码下拉数据源（来自数据库 market_data）。 */
const symbolList = ref<string[]>([])
/** 多选结果以逗号分隔写回 `formConfig.symbols`（保持后端字符串格式）。 */
const selectedSymbols = computed<string[]>({
  get: () => formConfig.symbols.split(',').map((s) => s.trim()).filter(Boolean),
  set: (v) => { formConfig.symbols = v.join(',') },
})

onMounted(async () => {
  try { symbolList.value = await getSymbols() } catch { symbolList.value = [] }
})

const rules = {
  strategyId: [{ required: true, message: '请选择策略', trigger: 'change' }],
  startDate: [{ required: true, message: '请选择开始日期', trigger: 'change' }],
  endDate: [{ required: true, message: '请选择结束日期', trigger: 'change' }],
  initialCapital: [
    { required: true, message: '请输入初始资金', trigger: 'blur' },
    { type: 'number', min: 10000, message: '初始资金不能少于10,000', trigger: 'blur' },
  ],
  commissionRate: [
    { required: true, message: '请输入手续费率', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '手续费率应在0-0.1之间', trigger: 'blur' },
  ],
  slippage: [
    { required: true, message: '请输入滑点', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '滑点应在0-0.1之间', trigger: 'blur' },
  ],
  symbols: [{ required: true, message: '请选择标的代码', trigger: 'change' }],
}

function onStrategyChange(strategyId: string) {
  const strategy = props.strategies.find((s) => s.strategy_id === strategyId)
  if (strategy) {
    formConfig.strategyName = strategy.strategy_name
  }
}

async function handleRun() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  if (!formConfig.startDate || !formConfig.endDate) {
    ElMessage.warning('请选择回测时间范围')
    return
  }
  emit('run', { ...formConfig })
}

function handleSaveTemplate() {
  emit('saveTemplate', { ...formConfig })
}

function handleLoadTemplate(index: number) {
  emit('loadTemplate', index)
}

function handleReset() {
  Object.assign(formConfig, defaultConfig())
  emit('reset')
}

function setConfig(config: BacktestConfigData) {
  Object.assign(formConfig, config)
}

defineExpose({ setConfig })
</script>

<style scoped>

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.form-actions {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 8px;
  width: 100%;
}
</style>
