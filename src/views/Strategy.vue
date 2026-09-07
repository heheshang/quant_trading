<template>
  <div class="strategy-management page-stack">
    <header class="strategy-heading">
      <div>
        <span class="page-eyebrow">STRATEGY LIBRARY</span>
        <h2>策略管理</h2>
        <p class="page-description">从策略配置到回测验证，在一个工作空间内完成。</p>
      </div>
      <div class="controls">
        <el-button @click="store.fetchStrategies(true)" :loading="store.loading.list"><el-icon><Refresh /></el-icon><span>刷新</span></el-button>
        <el-button type="primary" @click="openNewStrategyDialog"><el-icon><Plus /></el-icon><span>新建策略</span></el-button>
      </div>
    </header>

    <el-card class="strategy-list-card" shadow="never">
      <template #header>
        <div class="card-header">
          <div class="library-title"><h3>策略库</h3><span class="strategy-count">{{ store.strategies.length }}</span></div>
          <span class="library-caption">配置 · 回测 · 执行</span>
        </div>
      </template>

      <StrategyTable
        :strategies="store.strategies"
        :strategy-types="store.strategyTypes"
        :loading="store.isAnyLoading"
        :search-query="searchQuery"
        :active-filters="activeFilters"
        :page-size="pageSize"
        :current-page="currentPage"
        :selected="selectedStrategies"
        @update:search-query="searchQuery = $event"
        @update:active-filters="activeFilters = $event"
        @update:page-size="pageSize = $event"
        @update:current-page="currentPage = $event"
        @update:selected="selectedStrategies = $event"
        @search="onSearch"
        @toggle="toggleStrategyStatus"
        @detail="openDetailPanel"
        @edit="openEditDialog"
        @delete="confirmDeleteStrategy"
        @backtest="runBacktest"
        @optimize="openOptimizer"
        @lifecycle="handleLifecycle"
        @batch-start="batchStart"
        @batch-stop="batchStop"
        @batch-delete="batchDelete"
      />
    </el-card>
    <p class="risk-note"><el-icon><InfoFilled /></el-icon>交易前请核对持仓与亏损限制；回测表现不代表未来收益。</p>

    <el-drawer v-model="detailPanelVisible" title="策略详情" size="min(600px, 100vw)">
      <StrategyDetailPanel
        v-if="detailStrategy"
        :strategy-id="detailStrategy.strategy_id"
        :status="detailStrategy.status"
        :description="detailStrategy.description || '暂无策略描述'"
        :tags="detailStrategy.tags || []"
        :symbols="detailStrategy.symbols || []"
        :params-values="detailStrategy.params || {}"
        :strategy-type="detailStrategy.strategy_type"
        :instance-label="detailStrategy.instance_label || ''"
        :create-time="new Date(detailStrategy.created_at).getTime()"
        :update-time="new Date(detailStrategy.updated_at).getTime()"
        :is-running="detailStrategy.status === 'Running'"
        @edit="openEditDialog(detailStrategy!); detailPanelVisible = false"
        @start="handleLifecycle('start', detailStrategy!)"
        @stop="handleLifecycle('stop', detailStrategy!)"
        @refresh="store.fetchStrategies(true)"
      />
    </el-drawer>

    <StrategyFormDialog
      v-model:visible="dialogVisible"
      :strategy="editingStrategy"
      @saved="onStrategySaved"
    />

    <BacktestConfigDialog
      v-model:visible="backtestConfigDialogVisible"
      :strategy-name="pendingStrategyName"
      :loading="backtestLoading"
      @confirm="handleBacktestConfigConfirmed"
    />

    <StrategyOptimizerDialog
      v-model:visible="optimizerDialogVisible"
      :strategy-id="optimizerStrategy?.strategy_id"
      :strategy-name="optimizerStrategy?.strategy_name"
    />

    <ConfirmDialog
      v-model:visible="deleteDialogVisible"
      title="确认删除"
      message="确定要删除这个策略吗？此操作不可撤销。"
      type="danger"
      confirm-text="删除"
      @confirm="executeDelete"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { ElMessage } from 'element-plus';
import { useStrategyStore } from '@/stores/strategy';
import { useStrategyLifecycleStore } from '@/stores/strategyLifecycle';
import { runBacktest as apiRunBacktest } from '@/services/backtest';
import type { StrategyParams } from '@/services/types';
import StrategyTable from '@/components/strategy/StrategyTable.vue';
import StrategyDetailPanel from '@/components/strategy/StrategyDetailPanel.vue';
import BacktestConfigDialog from '@/components/backtest/BacktestConfigDialog.vue';
import type { BacktestRunParams } from '@/components/backtest/BacktestConfigDialog.vue';
import StrategyOptimizerDialog from '@/components/strategy/StrategyOptimizerDialog.vue';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';

const store = useStrategyStore();
const lifecycleStore = useStrategyLifecycleStore();

// --- Table state ---
const searchQuery = ref('');
const activeFilters = ref<Record<string, any>>({});
const currentPage = ref(1);
const pageSize = ref(10);
const selectedStrategies = ref<StrategyParams[]>([]);

// --- Form dialog ---
const dialogVisible = ref(false);
const editingStrategy = ref<StrategyParams | null>(null);

function openNewStrategyDialog() {
  editingStrategy.value = null;
  dialogVisible.value = true;
}

function openEditDialog(strategy: StrategyParams) {
  editingStrategy.value = strategy;
  dialogVisible.value = true;
}

function onStrategySaved() {
  store.fetchStrategies(true);
}

function onSearch() {
  store.fetchStrategies(true);
}

async function toggleStrategyStatus(strategyId: string, enabled: boolean) {
  await lifecycleStore.toggleStrategy(strategyId, enabled);
}

// --- Detail panel ---
const detailPanelVisible = ref(false);
const detailStrategy = ref<StrategyParams | null>(null);

function openDetailPanel(strategy: StrategyParams) {
  detailStrategy.value = strategy;
  detailPanelVisible.value = true;
}

const backtestConfigDialogVisible = ref(false);
const pendingBacktestStrategyId = ref('');
const pendingStrategyName = ref('');
const backtestDialogVisible = ref(false);
const backtestResult = ref<Record<string, unknown> | null>(null);
const backtestLoading = ref(false);
const optimizerDialogVisible = ref(false);
const optimizerStrategy = ref<StrategyParams | null>(null);

function openOptimizer(strategy: StrategyParams) {
  optimizerStrategy.value = strategy;
  optimizerDialogVisible.value = true;
}

function runBacktest(strategyId: string) {
  pendingBacktestStrategyId.value = strategyId;
  const strategy = store.strategies.find((s) => s.strategy_id === strategyId);
  pendingStrategyName.value = strategy?.strategy_name ?? '';
  backtestConfigDialogVisible.value = true;
}

async function handleBacktestConfigConfirmed(params: BacktestRunParams) {
  if (!pendingBacktestStrategyId.value) return;
  const strategyId = pendingBacktestStrategyId.value;
  backtestConfigDialogVisible.value = false;
  try {
    backtestLoading.value = true;
    const symbols = params.symbols
      .split(',')
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    const result = await apiRunBacktest(
      strategyId,
      params.startDate,
      params.endDate,
      params.initialCapital,
      params.commissionRate,
      params.slippage,
      symbols,
      '1H',
    );
    backtestResult.value = result as unknown as Record<string, unknown>;
    backtestDialogVisible.value = true;
  } catch {
    ElMessage.error('回测失败');
  } finally {
    backtestLoading.value = false;
  }
}
// --- Batch operations ---
async function batchStart(strategies: StrategyParams[]) {
  for (const s of strategies) {
    try { await lifecycleStore.startStrategy(s.strategy_id); ElMessage.success(`已启动: ${s.strategy_name}`); }
    catch { ElMessage.error(`启动失败: ${s.strategy_name}`); }
  }
  selectedStrategies.value = [];
}

async function batchStop(strategies: StrategyParams[]) {
  for (const s of strategies) {
    try { await lifecycleStore.stopStrategy(s.strategy_id); ElMessage.success(`已停止: ${s.strategy_name}`); }
    catch { ElMessage.error(`停止失败: ${s.strategy_name}`); }
  }
  selectedStrategies.value = [];
}

async function batchDelete(strategies: StrategyParams[]) {
  for (const s of strategies) {
    try { await store.deleteStrategy(s.strategy_id); ElMessage.success(`已删除: ${s.strategy_name}`); }
    catch { ElMessage.error(`删除失败: ${s.strategy_name}`); }
  }
  selectedStrategies.value = [];
}

// --- Lifecycle ---
const lifecycleApiMap: Record<string, (id: string) => Promise<string>> = {
  deploy: (id) => lifecycleStore.deployStrategy(id).then(() => 'deployed'),
  start: (id) => lifecycleStore.startStrategy(id).then(() => 'started'),
  stop: (id) => lifecycleStore.stopStrategy(id).then(() => 'stopped'),
  pause: (id) => lifecycleStore.pauseStrategy(id).then(() => 'paused'),
  resume: (id) => lifecycleStore.resumeStrategy(id).then(() => 'resumed'),
  archive: (id) => lifecycleStore.archiveStrategy(id).then(() => 'archived'),
};

const actionLabels: Record<string, string> = {
  deploy: '部署', start: '启动', stop: '停止',
  pause: '暂停', resume: '恢复', archive: '归档',
};

async function handleLifecycle(action: string, strategy: StrategyParams) {
  const api = lifecycleApiMap[action];
  if (!api) { ElMessage.error('未知操作'); return; }
  try {
    await api(strategy.strategy_id);
    ElMessage.success(`策略${actionLabels[action]}成功`);
  } catch {
    ElMessage.error(`策略${actionLabels[action]}失败`);
  }
}

// --- Delete ---
const deleteDialogVisible = ref(false);
const strategyToDelete = ref<string | null>(null);

function confirmDeleteStrategy(strategyId: string) {
  strategyToDelete.value = strategyId;
  deleteDialogVisible.value = true;
}

async function executeDelete() {
  if (!strategyToDelete.value) return;
  try {
    await store.deleteStrategy(strategyToDelete.value);
    ElMessage.success('策略删除成功');
    deleteDialogVisible.value = false;
    strategyToDelete.value = null;
  } catch {
    ElMessage.error('删除策略失败');
  }
}

defineExpose({
  store,
  lifecycleStore,
  searchQuery,
  activeFilters,
  currentPage,
  pageSize,
  selectedStrategies,
  dialogVisible,
  editingStrategy,
  detailPanelVisible,
  detailStrategy,
  backtestConfigDialogVisible,
  pendingBacktestStrategyId,
  pendingStrategyName,
  backtestDialogVisible,
  backtestResult,
  backtestLoading,
  deleteDialogVisible,
  strategyToDelete,
  openNewStrategyDialog,
  openEditDialog,
  onStrategySaved,
  onSearch,
  toggleStrategyStatus,
  openDetailPanel,
  runBacktest,
  handleBacktestConfigConfirmed,
  batchStart,
  batchStop,
  batchDelete,
  handleLifecycle,
  confirmDeleteStrategy,
  executeDelete,
})
// --- Init ---
onMounted(() => {
  store.fetchStrategies();
  store.listStrategyTypes();
});
</script>

<style scoped>
.strategy-management { min-width: 0; gap: 28px; }
.strategy-heading { display: flex; align-items: center; justify-content: space-between; gap: 24px; padding: 8px 0; }
.page-eyebrow { color: var(--color-primary); font-size: 11px; letter-spacing: 0.16em; font-weight: 700; }
.strategy-heading h2 { margin-top: 8px; font-size: var(--font-size-xxl); line-height: 1.4; letter-spacing: -0.03em; }
.page-description { margin-top: 8px; color: var(--color-text-secondary); font-size: 14px; line-height: 1.7; }
.controls { display: flex; flex-shrink: 0; gap: 10px; }
.controls .el-button { margin-left: 0; }
.controls .el-icon { margin-right: 6px; }
.card-header, .library-title { display: flex; align-items: center; gap: 12px; }
.card-header { justify-content: space-between; }
.library-title h3 { font-size: 16px; font-weight: 600; }
.strategy-count { padding: 2px 9px; border-radius: 6px; background: var(--el-color-primary-light-9); color: var(--color-primary); font-size: 12px; font-variant-numeric: tabular-nums; }
.library-caption { color: var(--color-text-secondary); font-size: 12px; font-weight: 400; }
.risk-note { display: flex; align-items: flex-start; gap: 8px; color: var(--color-text-secondary); font-size: 12px; line-height: 1.7; }
.risk-note .el-icon { margin-top: 3px; flex-shrink: 0; }
@media (max-width: 767px) {
  .strategy-management { gap: 20px; }
  .strategy-heading { align-items: stretch; flex-direction: column; gap: 20px; }
  .strategy-heading h2 { font-size: 26px; }
  .controls .el-button { flex: 1; }
  .library-caption { display: none; }
}
</style>
