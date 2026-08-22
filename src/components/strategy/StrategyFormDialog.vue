<template>
  <el-dialog
    v-model="localVisible"
    :title="isEditing ? '编辑策略' : '新建策略'"
    width="680px"
    top="5vh"
    @closed="handleClosed"
  >
    <el-form :model="formData" label-position="top" :rules="rules" ref="formRef">
      <!-- Section: 基本信息 -->
      <div class="form-section">
        <div class="section-header">
          <el-icon><InfoFilled /></el-icon>
          <span>基本信息</span>
        </div>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="策略名称" prop="strategy_name">
              <el-input v-model="formData.strategy_name" placeholder="输入策略名称，2-50个字符" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="实例标签">
              <el-input v-model="formData.instance_label" placeholder="同一类型多实例时的标识（可选）" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="策略类型" prop="strategy_type">
          <el-select v-model="formData.strategy_type" placeholder="请选择策略类型" @change="onTypeChange" style="width: 100%">
            <el-option
              v-for="t in strategyTypes"
              :key="t.type_name"
              :label="t.display_name || t.type_name"
              :value="t.type_name"
            >
              <span>{{ t.display_name || t.type_name }}</span>
              <span v-if="t.description" class="option-desc">{{ t.description }}</span>
            </el-option>
          </el-select>
          <p v-if="selectedTypeDesc" class="field-hint">{{ selectedTypeDesc }}</p>
        </el-form-item>
      </div>

      <el-divider />

      <!-- Section: 风控设置 -->
      <div class="form-section">
        <div class="section-header">
          <el-icon><WarningFilled /></el-icon>
          <span>风控设置</span>
        </div>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="最大持仓（¥）" prop="max_position">
              <el-input-number v-model="formData.max_position" :min="0" :step="10000" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大日亏损（¥）" prop="max_daily_loss">
              <el-input-number v-model="formData.max_daily_loss" :min="0" :step="1000" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="启用状态">
          <el-switch v-model="formData.enabled" active-text="创建后立即启用" inactive-text="创建为草稿" />
        </el-form-item>
      </div>

      <el-divider />

      <!-- Section: 策略详情 -->
      <div class="form-section">
        <div class="section-header">
          <el-icon><EditPen /></el-icon>
          <span>策略详情</span>
        </div>
        <el-form-item label="描述" prop="description">
          <el-input v-model="formData.description" type="textarea" :rows="2" placeholder="描述策略逻辑、适用场景等（可选）" />
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="标签" prop="tags">
              <el-select v-model="formData.tags" multiple allow-create filterable default-first-option placeholder="输入标签后回车添加" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="交易标的" prop="symbols">
              <el-select v-model="formData.symbols" multiple allow-create filterable default-first-option placeholder="输入交易标的后回车添加" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
      </div>

      <el-divider />

      <!-- Section: 策略参数 -->
      <div class="form-section">
        <div class="section-header">
          <el-icon><Setting /></el-icon>
          <span>策略参数</span>
        </div>
        <div v-if="currentParamSchema.length">
          <p class="field-hint" style="margin-bottom: 12px;">配置策略运行所需的参数，修改后将立即反映在预览中</p>
          <StrategyParamEditor v-model="strategyParams" :schema="currentParamSchema" />
        </div>
        <el-empty v-else description="请先选择策略类型以加载参数模板" :image-size="80" />
      </div>
    </el-form>

    <template #footer>
      <span class="dialog-footer">
        <el-button @click="localVisible = false">取消</el-button>
        <el-button type="primary" @click="save" :loading="submitting">保存</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance } from 'element-plus';
import { InfoFilled, WarningFilled, EditPen, Setting } from '@element-plus/icons-vue';
import { useStrategyStore } from '@/stores/strategy';
import { useAuthStore } from '@/stores/auth';
import StrategyParamEditor from './StrategyParamEditor.vue';
import type { StrategyParams, StrategyStatus, ParameterSchema } from '@/services/types';

const props = withDefaults(defineProps<{
  visible: boolean;
  strategy?: StrategyParams | null;
}>(), {
  strategy: null,
});

const emit = defineEmits<{
  'update:visible': [value: boolean];
  'saved': [];
}>();

const store = useStrategyStore();
const authStore = useAuthStore();
const formRef = ref<FormInstance>();
const submitting = ref(false);
const strategyParams = ref<Record<string, unknown>>({});

const localVisible = computed({
  get: () => props.visible,
  set: (v) => emit('update:visible', v),
});

const isEditing = computed(() => !!props.strategy?.strategy_id);

const strategyTypes = computed(() => store.strategyTypes);

const selectedTypeDesc = computed(() => {
  const t = store.strategyTypes.find(x => x.type_name === formData.value.strategy_type);
  return t?.description || '';
});

const currentParamSchema = computed<ParameterSchema[]>(() => {
  const t = store.strategyTypes.find(x => x.type_name === formData.value.strategy_type);
  return (t?.parameters && Array.isArray(t.parameters)) ? t.parameters : [];
});

const rules = {
  strategy_name: [
    { required: true, message: '请输入策略名称', trigger: 'blur' },
    { min: 2, max: 50, message: '策略名称长度应在2-50个字符之间', trigger: 'blur' },
  ],
  strategy_type: [
    { required: true, message: '请选择策略类型', trigger: 'change' },
  ],
  max_position: [
    { required: true, message: '请输入最大持仓', trigger: 'blur' },
    { type: 'number' as const, min: 0, message: '最大持仓不能小于0', trigger: 'blur' },
  ],
  max_daily_loss: [
    { required: true, message: '请输入最大日亏损', trigger: 'blur' },
    { type: 'number' as const, min: 0, message: '最大日亏损不能小于0', trigger: 'blur' },
  ],
};

function defaultFormData() {
  return {
    strategy_id: '',
    strategy_name: '',
    strategy_type: store.strategyTypes[0]?.type_name || 'TrendFollowing',
    instance_label: '',
    enabled: true,
    max_position: 100000,
    max_daily_loss: 5000,
    status: 'Draft',
    description: '',
    tags: [] as string[],
    symbols: [] as string[],
    params: {} as Record<string, unknown>,
    created_at: '',
    updated_at: '',
  };
}

const formData = ref(defaultFormData());

watch(() => props.visible, (visible) => {
  if (!visible) return;
  if (props.strategy) {
    formData.value = {
      strategy_id: props.strategy.strategy_id,
      strategy_name: props.strategy.strategy_name,
      strategy_type: props.strategy.strategy_type,
      instance_label: props.strategy.instance_label || '',
      enabled: props.strategy.enabled ?? true,
      max_position: props.strategy.max_position ?? 100000,
      max_daily_loss: props.strategy.max_daily_loss ?? 5000,
      status: props.strategy.status || 'Draft',
      description: props.strategy.description || '',
      tags: [...(props.strategy.tags || [])],
      symbols: [...(props.strategy.symbols || [])],
      params: {},
      created_at: props.strategy.created_at || '',
      updated_at: props.strategy.updated_at || '',
    };
    strategyParams.value = props.strategy.params
      ? JSON.parse(JSON.stringify(props.strategy.params))
      : {};
  } else {
    formData.value = defaultFormData();
    const firstType = store.strategyTypes[0];
    if (firstType?.parameters) {
      const defaults: Record<string, unknown> = {};
      for (const s of firstType.parameters) defaults[s.name] = s.default;
      strategyParams.value = defaults;
    } else {
      strategyParams.value = {};
    }
  }
});

async function onTypeChange(typeName: string) {
  if (isEditing.value && props.strategy && typeName !== props.strategy.strategy_type) {
    try {
      await ElMessageBox.confirm(
        '切换策略类型将重置参数为默认值，确认切换？',
        '确认',
        { confirmButtonText: '确认切换', cancelButtonText: '取消', type: 'warning' },
      );
    } catch {
      formData.value.strategy_type = props.strategy.strategy_type;
      return;
    }
  }
  strategyParams.value = {};
  const typeInfo = store.strategyTypes.find(t => t.type_name === typeName);
  if (typeInfo) {
    const defaults: Record<string, unknown> = {};
    for (const s of typeInfo.parameters) defaults[s.name] = s.default;
    strategyParams.value = defaults;
  }
}

async function save() {
  if (!formRef.value) return;
  try {
    const valid = await formRef.value.validate();
    if (!valid) return;
  } catch { return; }

  submitting.value = true;
  try {
    if (isEditing.value) {
      await store.updateStrategy({
        ...formData.value,
        params: strategyParams.value,
        strategy_type: formData.value.strategy_type as any,
        status: formData.value.status as StrategyStatus,
      } as StrategyParams);
    } else {
      if (!authStore.currentUser) {
        ElMessage.error('请先登录');
        return;
      }
      await store.createNewStrategy(
        formData.value.strategy_type,
        formData.value.strategy_name,
        strategyParams.value,
        formData.value.enabled,
        formData.value.max_position,
        formData.value.max_daily_loss,
        authStore.currentUser.id,
        formData.value.instance_label || undefined,
        formData.value.description || undefined,
        formData.value.tags?.length ? formData.value.tags : undefined,
        formData.value.symbols?.length ? formData.value.symbols : undefined,
      );
    }
    ElMessage.success('策略保存成功');
    emit('saved');
    localVisible.value = false;
  } catch {
    ElMessage.error('保存策略失败');
  } finally {
    submitting.value = false;
  }
}

function handleClosed() {
  formRef.value?.resetFields();
}
</script>

<style scoped>
.form-section { padding: 0 4px; }
.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin-bottom: 16px;
}
.section-header .el-icon { font-size: 18px; color: #409eff; }
.field-hint {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--color-text-secondary);
  line-height: 1.4;
}
.dialog-footer { display: flex; justify-content: flex-end; gap: 10px; }
:deep(.el-select .el-select-dropdown__item .option-desc) {
  display: block;
  font-size: 12px;
  color: var(--color-text-secondary);
  white-space: normal;
  line-height: 1.3;
  margin-top: 2px;
}
:deep(.el-divider) { margin: 20px 0; }
:deep(.el-empty) { padding: 24px 0; }
</style>
