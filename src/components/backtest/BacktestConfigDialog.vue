<template>
  <el-dialog
    :model-value="visible"
    title="回测配置"
    width="540px"
    :close-on-click-modal="false"
    @update:model-value="emit('update:visible', $event)"
  >
    <el-form :model="form" :rules="rules" ref="formRef" label-width="100px">
      <el-form-item label="策略">
        <el-input :model-value="strategyName" readonly />
      </el-form-item>
      <el-form-item label="初始资金" prop="initialCapital">
        <el-input-number v-model="form.initialCapital" :min="10000" :step="100000" style="width: 100%" />
      </el-form-item>
      <el-form-item label="手续费率" prop="commissionRate">
        <el-input-number v-model="form.commissionRate" :min="0" :max="0.1" :step="0.001" style="width: 100%" />
      </el-form-item>
      <el-form-item label="滑点" prop="slippage">
        <el-input-number v-model="form.slippage" :min="0" :max="0.1" :step="0.001" style="width: 100%" />
      </el-form-item>
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
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="loading" @click="handleConfirm">开始回测</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, ref, watch, computed, onMounted } from 'vue'
import type { FormInstance } from 'element-plus'
import { ElMessage } from 'element-plus'
import { getSymbols } from '@/services/market'

export interface BacktestRunParams {
  startDate: string
  endDate: string
  initialCapital: number
  commissionRate: number
  slippage: number
  symbols: string
}

const props = withDefaults(
  defineProps<{
    visible: boolean
    strategyName?: string
    loading?: boolean
  }>(),
  {
    strategyName: '',
    loading: false,
  },
)

const emit = defineEmits<{
  'update:visible': [value: boolean]
  confirm: [params: BacktestRunParams]
}>()

const formRef = ref<FormInstance>()
const form = reactive<BacktestRunParams>(defaultParams())
const symbolList = ref<string[]>([])
const selectedSymbols = computed<string[]>({
  get: () => form.symbols.split(',').map((s) => s.trim()).filter(Boolean),
  set: (v) => { form.symbols = v.join(',') },
})

onMounted(async () => {
  try { symbolList.value = await getSymbols() } catch { symbolList.value = [] }
})

function defaultParams(): BacktestRunParams {
  const today = new Date()
  const oneMonthAgo = new Date()
  oneMonthAgo.setMonth(oneMonthAgo.getMonth() - 1)
  return {
    startDate: oneMonthAgo.toISOString().split('T')[0],
    endDate: today.toISOString().split('T')[0],
    initialCapital: 1000000,
    commissionRate: 0.0003,
    slippage: 0.0001,
    symbols: 'BTC-USDT',
  }
}

const rules = {
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

// Reset form params each time the dialog opens so a new run starts from defaults.
watch(
  () => props.visible,
  (visible) => {
    if (visible) Object.assign(form, defaultParams())
  },
)

async function handleConfirm() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  if (!form.startDate || !form.endDate) {
    ElMessage.warning('请选择回测时间范围')
    return
  }
  emit('confirm', { ...form })
}

defineExpose({ form, formRef })
</script>
