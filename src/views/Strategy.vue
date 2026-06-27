<template>
  <div class="strategy-management">
    <el-row :gutter="20" class="header">
      <el-col :span="18">
        <h2>策略管理</h2>
      </el-col>
      <el-col :span="6" class="controls">
        <el-button type="primary" @click="openStrategyDialog()">新建策略</el-button>
        <el-button @click="store.fetchStrategies(true)" :loading="store.loading">刷新</el-button>
      </el-col>
    </el-row>

    <el-card class="strategy-list-card">
      <template #header>
        <div class="card-header">
          <span>策略列表</span>
          <div class="card-header-controls">
            <SearchBar v-model="searchQuery" placeholder="搜索策略名称" @search="onSearch" />
          </div>
        </div>
      </template>

      <div class="table-toolbar">
        <FilterPanel
          v-model="activeFilters"
          :filters="filterOptions"
          @change="onFilterChange"
        />
        <div class="batch-actions" v-if="selectedStrategies.length > 0">
          <el-button size="small" @click="batchStart">批量启动</el-button>
          <el-button size="small" @click="batchStop">批量停止</el-button>
          <el-button size="small" type="danger" @click="batchDelete">批量删除</el-button>
        </div>
      </div>
      
      <el-table
        v-if="paginatedStrategies.length > 0"
        :data="paginatedStrategies"
        style="width: 100%"
        v-loading="store.loading"
        @selection-change="onSelectionChange"
      >
        <el-table-column type="selection" width="50" />
        <el-table-column prop="strategy_name" label="策略名称" width="180" />
        <el-table-column prop="strategy_type" label="策略类型" width="120">
          <template #default="scope">
            {{ getStrategyTypeText(scope.row.strategy_type) }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="120">
          <template #default="scope">
            <StrategyStatusTag :status="getStatusTag(scope.row)" size="small" />
          </template>
        </el-table-column>
        <el-table-column label="启用" width="80">
          <template #default="scope">
            <el-switch
              v-model="scope.row.enabled"
              @change="toggleStrategyStatus(scope.row)"
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
            <el-dropdown trigger="click" @command="(cmd: string) => handleLifecycle(cmd, scope.row)">
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
            <el-button size="small" @click="openDetailPanel(scope.row)">详情</el-button>
            <el-button size="small" @click="openStrategyDialog(scope.row)">编辑</el-button>
            <el-button size="small" type="primary" @click="runBacktest(scope.row.strategy_id)">回测</el-button>
            <el-button size="small" type="danger" @click="confirmDeleteStrategy(scope.row.strategy_id)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div v-if="store.strategies.length > 0" class="table-footer">
        <Paginator
          :total="store.strategies.length"
          :page-size="pageSize"
          :current-page="currentPage"
          @update:current-page="currentPage = $event"
          @update:page-size="pageSize = $event"
        />
      </div>
      <EmptyState v-else-if="!store.loading" title="暂无策略" description="点击「新建策略」按钮创建第一个策略" />
    </el-card>

    <!-- 策略详情面板 -->
    <el-drawer v-model="detailPanelVisible" title="策略详情" size="600px">
      <StrategyDetailPanel
        v-if="detailStrategy"
        :strategy-id="detailStrategy.strategy_id"
        :status="getStatusTag(detailStrategy)"
        :description="detailStrategy.description || '暂无策略描述'"
        :tags="detailStrategy.tags || []"
        :strategy-type="detailStrategy.strategy_type"
        :create-time="new Date(detailStrategy.created_at).getTime()"
        :update-time="new Date(detailStrategy.updated_at).getTime()"
        :is-running="isRunningStatus(detailStrategy)"
        @edit="openStrategyDialog(detailStrategy!); detailPanelVisible = false"
        @start="handleLifecycle('start', detailStrategy!)"
        @stop="handleLifecycle('stop', detailStrategy!)"
        @refresh="store.fetchStrategies(true)"
      />
    </el-drawer>

    <!-- 策略编辑对话框 -->
    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="600px">
      <el-form :model="currentStrategy" label-width="120px" :rules="strategyRules" ref="strategyFormRef">
        <el-form-item label="策略名称" prop="strategy_name">
          <el-input v-model="currentStrategy.strategy_name" />
        </el-form-item>
        
        <el-form-item label="策略类型" prop="strategy_type">
          <el-select v-model="currentStrategy.strategy_type" placeholder="请选择策略类型">
            <el-option label="趋势跟踪" value="TrendFollowing" />
            <el-option label="均值回归" value="MeanReversion" />
            <el-option label="套利" value="Arbitrage" />
            <el-option label="做市" value="MarketMaking" />
            <el-option label="统计套利" value="Statistical" />
            <el-option label="机器学习" value="MachineLearning" />
            <el-option label="自定义" value="Custom" />
          </el-select>
        </el-form-item>
        
        <el-form-item label="最大持仓" prop="max_position">
          <el-input-number v-model="currentStrategy.max_position" :min="0" :step="10000" />
        </el-form-item>
        
        <el-form-item label="最大日亏损" prop="max_daily_loss">
          <el-input-number v-model="currentStrategy.max_daily_loss" :min="0" :step="1000" />
        </el-form-item>
        
        <el-form-item label="启用状态">
          <el-switch v-model="currentStrategy.enabled" />
        </el-form-item>
        
        <el-form-item label="策略参数">
          <div class="strategy-params">
            <div v-for="(_value, key) in strategyParams" :key="key" class="param-item">
              <el-input v-model="strategyParams[key]" :placeholder="key">
                <template #prepend>{{ key }}</template>
              </el-input>
            </div>
            <el-button type="primary" @click="addParam" size="small">添加参数</el-button>
          </div>
        </el-form-item>
      </el-form>
      
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="dialogVisible = false">取消</el-button>
          <el-button type="primary" @click="saveStrategy">保存</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 回测结果对话框 -->
    <el-dialog v-model="backtestDialogVisible" title="回测结果" width="800px">
      <div v-if="backtestResult">
        <el-row :gutter="20">
          <el-col :span="8">
            <el-card class="backtest-stat-card">
              <div class="stat-item">
                <div class="stat-label">总收益率</div>
                <div class="stat-value" :class="{ positive: backtestResult.total_return > 0, negative: backtestResult.total_return < 0 }">
                  {{ formatPercentage(backtestResult.total_return) }}
                </div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card class="backtest-stat-card">
              <div class="stat-item">
                <div class="stat-label">夏普比率</div>
                <div class="stat-value">{{ backtestResult.sharpe_ratio.toFixed(2) }}</div>
              </div>
            </el-card>
          </el-col>
          <el-col :span="8">
            <el-card class="backtest-stat-card">
              <div class="stat-item">
                <div class="stat-label">最大回撤</div>
                <div class="stat-value negative">{{ formatPercentage(backtestResult.max_drawdown) }}</div>
              </div>
            </el-card>
          </el-col>
        </el-row>
        
        <el-row :gutter="20" style="margin-top: 20px;">
          <el-col :span="24">
            <PerformanceChart
              :equity-curve="backtestResult.equity_curve?.map((pair: [string, number]) => pair[1]) || []"
              :show-controls="false"
              height="300px"
            />
          </el-col>
        </el-row>
      </div>
      
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="backtestDialogVisible = false">关闭</el-button>
        </span>
      </template>
    </el-dialog>

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
import { ref, onMounted, computed } from 'vue';
import { useStrategyStore } from '@/stores/strategy';
import { runBacktest as apiRunBacktest } from '@/services/api';
import StrategyStatusTag from '@/components/strategy/StrategyStatusTag.vue';
import StrategyDetailPanel from '@/components/strategy/StrategyDetailPanel.vue';
import PerformanceChart from '@/components/strategy/PerformanceChart.vue';
import { ElMessage, type FormInstance } from 'element-plus';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import EmptyState from '@/components/common/EmptyState.vue';
import SearchBar from '@/components/common/SearchBar.vue';
import FilterPanel from '@/components/common/FilterPanel.vue';
import Paginator from '@/components/common/Paginator.vue';
import { ArrowDown } from '@element-plus/icons-vue';
import type { StrategyParams, StrategyStatus } from '@/services/types';

const store = useStrategyStore();

const strategyFormRef = ref<FormInstance>();
const strategyRules = {
  strategy_name: [
    { required: true, message: '请输入策略名称', trigger: 'blur' },
    { min: 2, max: 50, message: '策略名称长度应在2-50个字符之间', trigger: 'blur' }
  ],
  strategy_type: [
    { required: true, message: '请选择策略类型', trigger: 'change' }
  ],
  max_position: [
    { required: true, message: '请输入最大持仓', trigger: 'blur' },
    { type: 'number', min: 0, message: '最大持仓不能小于0', trigger: 'blur' }
  ],
  max_daily_loss: [
    { required: true, message: '请输入最大日亏损', trigger: 'blur' },
    { type: 'number', min: 0, message: '最大日亏损不能小于0', trigger: 'blur' }
  ]
};

const dialogVisible = ref(false);
const backtestDialogVisible = ref(false);
const currentStrategy = ref<StrategyParams>({} as StrategyParams);
const strategyParams = ref<Record<string, any>>({});
const backtestResult = ref<any>(null);
const isEditing = ref(false);
const deleteDialogVisible = ref(false);
const strategyToDelete = ref<string | null>(null);

const detailPanelVisible = ref(false);
const detailStrategy = ref<StrategyParams | null>(null);

const searchQuery = ref('');
const activeFilters = ref<Record<string, any>>({});
const currentPage = ref(1);
const pageSize = ref(10);
const selectedStrategies = ref<StrategyParams[]>([]);

const filterOptions = [
  { key: 'strategy_type', label: '策略类型', type: 'select' as const, options: ['TrendFollowing', 'MeanReversion', 'Arbitrage', 'MarketMaking', 'Statistical', 'MachineLearning', 'Custom'].map(v => ({ label: v, value: v })) },
];

const filteredStrategies = computed(() => {
  let list = store.strategies;
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    list = list.filter((s) => s.strategy_name?.toLowerCase().includes(q));
  }
  if (activeFilters.value.strategy_type) {
    list = list.filter((s) => s.strategy_type === activeFilters.value.strategy_type);
  }
  return list;
});

const paginatedStrategies = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value;
  return filteredStrategies.value.slice(start, start + pageSize.value);
});

const dialogTitle = computed(() => isEditing.value ? '编辑策略' : '新建策略');

function getStatusTag(strategy: StrategyParams): 'active' | 'inactive' | 'pending' | 'error' | 'warning' | 'draft' {
  const status = strategy.status as StrategyStatus | undefined;
  if (!status || status === 'Draft') return 'draft';
  if (status === 'Running') return 'active';
  if (status === 'Paused') return 'warning';
  if (status === 'Archived') return 'inactive';
  if (status === 'Deployed') return 'pending';
  if (status === 'Backtesting') return 'pending';
  return 'draft';
}

function isRunningStatus(strategy: StrategyParams): boolean {
  const status = strategy.status as StrategyStatus | undefined;
  return status === 'Running';
}

function onSearch() { currentPage.value = 1; }
function onFilterChange() { currentPage.value = 1; }
function onSelectionChange(rows: StrategyParams[]) { selectedStrategies.value = rows; }

async function batchStart() {
  for (const s of selectedStrategies.value) {
    try { await store.startStrategy(s.strategy_id); ElMessage.success(`已启动: ${s.strategy_name}`); }
    catch { ElMessage.error(`启动失败: ${s.strategy_name}`); }
  }
  selectedStrategies.value = [];
}

async function batchStop() {
  for (const s of selectedStrategies.value) {
    try { await store.stopStrategy(s.strategy_id); ElMessage.success(`已停止: ${s.strategy_name}`); }
    catch { ElMessage.error(`停止失败: ${s.strategy_name}`); }
  }
  selectedStrategies.value = [];
}

async function batchDelete() {
  for (const s of selectedStrategies.value) {
    try { await store.deleteStrategy(s.strategy_id); ElMessage.success(`已删除: ${s.strategy_name}`); }
    catch { ElMessage.error(`删除失败: ${s.strategy_name}`); }
  }
  selectedStrategies.value = [];
}

function formatCurrency(value: any): string {
  if (!value) return '0.00';
  return parseFloat(value.toString()).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function formatPercentage(value: any): string {
  if (!value) return '0.00%';
  return (parseFloat(value.toString()) * 100).toFixed(2) + '%';
}

function formatDate(dateString: string): string {
  return new Date(dateString).toLocaleString('zh-CN');
}

function getStrategyTypeText(type: string): string {
  const typeMap: Record<string, string> = {
    TrendFollowing: '趋势跟踪', MeanReversion: '均值回归', Arbitrage: '套利',
    MarketMaking: '做市', Statistical: '统计套利', MachineLearning: '机器学习', Custom: '自定义',
  };
  return typeMap[type] || type;
}

function openStrategyDialog(strategy?: StrategyParams) {
  if (strategy) {
    isEditing.value = true;
    currentStrategy.value = { ...strategy };
    strategyParams.value = strategy.params ? JSON.parse(JSON.stringify(strategy.params)) : {};
  } else {
    isEditing.value = false;
    currentStrategy.value = {
      strategy_id: '', strategy_name: '', strategy_type: 'TrendFollowing',
      params: {}, enabled: true, max_position: 100000, max_daily_loss: 5000,
      status: 'Draft' as StrategyStatus, description: '', tags: [], symbols: [],
      created_at: new Date().toISOString(), updated_at: new Date().toISOString(),
    };
    strategyParams.value = {};
  }
  dialogVisible.value = true;
}

function openDetailPanel(strategy: StrategyParams) {
  detailStrategy.value = strategy;
  detailPanelVisible.value = true;
}

function addParam() {
  const paramName = prompt('请输入参数名称:');
  if (paramName) {
    strategyParams.value[paramName] = '';
  }
}

async function saveStrategy() {
  if (!strategyFormRef.value) return;
  await strategyFormRef.value.validate(async (valid) => {
    if (!valid) return;
    try {
      currentStrategy.value.params = strategyParams.value;
      if (isEditing.value) {
        await store.updateStrategy(currentStrategy.value);
      } else {
        await store.createStrategy(currentStrategy.value);
      }
      ElMessage.success('策略保存成功');
      dialogVisible.value = false;
    } catch (err) {
      ElMessage.error('保存策略失败');
    }
  });
}

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

async function toggleStrategyStatus(strategy: StrategyParams) {
  try {
    await store.toggleStrategy(strategy.strategy_id, strategy.enabled);
    ElMessage.success('策略状态更新成功');
  } catch {
    strategy.enabled = !strategy.enabled;
    ElMessage.error('更新策略状态失败');
  }
}

const lifecycleApiMap: Record<string, (id: string) => Promise<string>> = {
  deploy: (id) => store.deployStrategy(id).then(() => 'deployed'),
  start: (id) => store.startStrategy(id).then(() => 'started'),
  stop: (id) => store.stopStrategy(id).then(() => 'stopped'),
  pause: (id) => store.pauseStrategy(id).then(() => 'paused'),
  resume: (id) => store.resumeStrategy(id).then(() => 'resumed'),
  archive: (id) => store.archiveStrategy(id).then(() => 'archived'),
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

async function runBacktest(strategyId: string) {
  try {
    store.loading = true;
    const now = new Date();
    const startDate = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
    const result = await apiRunBacktest(strategyId, startDate.toISOString(), now.toISOString(), 1000000, 0.0003, 0.0001, []);
    backtestResult.value = result;
    backtestDialogVisible.value = true;
  } catch {
    ElMessage.error('回测失败');
  } finally {
    store.loading = false;
  }
}

onMounted(() => {
  store.fetchStrategies();
});
</script>

<style scoped>
.strategy-management { padding: 20px; }
.header { margin-bottom: 20px; align-items: center; }
.controls { text-align: right; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.strategy-list-card { margin-bottom: 20px; }
.table-toolbar { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 12px; gap: 12px; }
.batch-actions { display: flex; gap: 8px; flex-shrink: 0; }
.table-footer { margin-top: 16px; display: flex; justify-content: flex-end; }
.card-header-controls { display: flex; align-items: center; gap: 8px; }
.strategy-params { width: 100%; }
.param-item { margin-bottom: 10px; }
.backtest-stat-card { margin-bottom: 20px; }
.stat-item { text-align: center; }
.stat-label { font-size: 14px; color: #999; margin-bottom: 8px; }
.stat-value { font-size: 20px; font-weight: bold; color: #333; }
.stat-value.positive { color: #67C23A; }
.stat-value.negative { color: #F56C6C; }
.dialog-footer { text-align: right; }
</style>
