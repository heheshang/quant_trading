<template>
  <div class="strategy-management">
    <el-row :gutter="20" class="header">
      <el-col :span="18">
        <h2>策略管理</h2>
      </el-col>
      <el-col :span="6" class="controls">
        <el-button type="primary" @click="openStrategyDialog()">新建策略</el-button>
        <el-button @click="fetchStrategies" :loading="loading">刷新</el-button>
      </el-col>
    </el-row>

    <!-- 策略列表 -->
    <el-card class="strategy-list-card">
      <template #header>
        <div class="card-header">
          <span>策略列表</span>
          <div class="card-header-controls">
            <SearchBar v-model="searchQuery" placeholder="搜索策略名称" @search="onSearch" />
          </div>
        </div>
      </template>

      <!-- Filters & batch actions -->
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
        v-loading="loading"
        @selection-change="onSelectionChange"
      >
        <el-table-column type="selection" width="50" />
        <el-table-column prop="strategy_name" label="策略名称" width="180" />
        <el-table-column prop="strategy_type" label="策略类型" width="120">
          <template #default="scope">
            {{ getStrategyTypeText(scope.row.strategy_type) }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="scope">
            <el-switch
              v-model="scope.row.enabled"
              @change="toggleStrategyStatus(scope.row)"
              active-text="启用"
              inactive-text="禁用"
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
        <el-table-column label="操作" width="240">
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
            <el-button size="small" @click="openStrategyDialog(scope.row)">编辑</el-button>
            <el-button size="small" type="primary" @click="runBacktest(scope.row.strategy_id)">回测</el-button>
            <el-button size="small" type="danger" @click="deleteStrategy(scope.row.strategy_id)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div v-if="strategies.length > 0" class="table-footer">
        <Paginator
          :total="strategies.length"
          :page-size="pageSize"
          :current-page="currentPage"
          @update:current-page="currentPage = $event"
          @update:page-size="pageSize = $event"
        />
      </div>
      <EmptyState v-else-if="!loading" title="暂无策略" description="点击「新建策略」按钮创建第一个策略" />
    </el-card>

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
        
        <!-- 策略参数配置 -->
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
            <el-card>
              <template #header>
                <div class="card-header">
                  <span>收益曲线</span>
                </div>
              </template>
              <div id="backtest-chart" style="height: 300px;"></div>
            </el-card>
          </el-col>
        </el-row>
      </div>
      
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="backtestDialogVisible = false">关闭</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- Delete strategy confirm dialog -->
    <ConfirmDialog
      v-model:visible="deleteDialogVisible"
      title="确认删除"
      message="确定要删除这个策略吗？此操作不可撤销。"
      type="danger"
      confirm-text="删除"
      @confirm="confirmDeleteStrategy"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { getStrategies, saveStrategy as apiSaveStrategy, deleteStrategy as apiDeleteStrategy, toggleStrategy, deployStrategy, startStrategy, stopStrategy, pauseStrategy, resumeStrategy, archiveStrategy, runBacktest as apiRunBacktest } from '@/services/api';
import * as echarts from 'echarts';
import { ElMessage, type FormInstance } from 'element-plus';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import EmptyState from '@/components/common/EmptyState.vue';
import SearchBar from '@/components/common/SearchBar.vue';
import FilterPanel from '@/components/common/FilterPanel.vue';
import Paginator from '@/components/common/Paginator.vue';
import { ArrowDown } from '@element-plus/icons-vue';

// Form validation
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

// Reactive data
const loading = ref(false);
const strategies = ref<any[]>([]);
const dialogVisible = ref(false);
const backtestDialogVisible = ref(false);
const currentStrategy = ref<any>({});
const strategyParams = ref<Record<string, any>>({});
const backtestResult = ref<any>(null);
const isEditing = ref(false);
const deleteDialogVisible = ref(false);
const strategyToDelete = ref<string | null>(null);

// ==================== Search / Filter / Pagination ====================
const searchQuery = ref('')
const activeFilters = ref<Record<string, any>>({})
const currentPage = ref(1)
const pageSize = ref(10)
const selectedStrategies = ref<any[]>([])

const filterOptions = [
  { key: 'strategy_type', label: '策略类型', type: 'select' as const, options: ['TrendFollowing', 'MeanReversion', 'Arbitrage', 'MarketMaking', 'Statistical', 'MachineLearning', 'Custom'].map(v => ({ label: v, value: v })) },
  { key: 'enabled', label: '状态', type: 'select' as const, options: ['enabled', 'disabled'].map(v => ({ label: v, value: v })) },
]

const filteredStrategies = computed(() => {
  let list = strategies.value
  // Search filter
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter((s: any) => s.strategy_name?.toLowerCase().includes(q))
  }
  // Active filters
  if (activeFilters.value.strategy_type) {
    list = list.filter((s: any) => s.strategy_type === activeFilters.value.strategy_type)
  }
  if (activeFilters.value.enabled) {
    const isEnabled = activeFilters.value.enabled === 'enabled'
    list = list.filter((s: any) => s.enabled === isEnabled)
  }
  return list
})

const paginatedStrategies = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredStrategies.value.slice(start, start + pageSize.value)
})

function onSearch() { currentPage.value = 1 }
function onFilterChange() { currentPage.value = 1 }
function onSelectionChange(rows: any[]) { selectedStrategies.value = rows }

async function batchStart() {
  for (const s of selectedStrategies.value) {
    try { await startStrategy(s.strategy_id); ElMessage.success(`已启动: ${s.strategy_name}`) }
    catch (e) { ElMessage.error(`启动失败: ${s.strategy_name}`) }
  }
  selectedStrategies.value = []
  fetchStrategies()
}

async function batchStop() {
  for (const s of selectedStrategies.value) {
    try { await stopStrategy(s.strategy_id); ElMessage.success(`已停止: ${s.strategy_name}`) }
    catch (e) { ElMessage.error(`停止失败: ${s.strategy_name}`) }
  }
  selectedStrategies.value = []
  fetchStrategies()
}

async function batchDelete() {
  for (const s of selectedStrategies.value) {
    try { await apiDeleteStrategy(s.strategy_id); ElMessage.success(`已删除: ${s.strategy_name}`) }
    catch (e) { ElMessage.error(`删除失败: ${s.strategy_name}`) }
  }
  selectedStrategies.value = []
  fetchStrategies()
}

// Computed properties
const dialogTitle = computed(() => isEditing.value ? '编辑策略' : '新建策略');

// Format currency
function formatCurrency(value: any): string {
  if (!value) return '0.00';
  return parseFloat(value.toString()).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

// Format percentage
function formatPercentage(value: any): string {
  if (!value) return '0.00%';
  return (parseFloat(value.toString()) * 100).toFixed(2) + '%';
}

// Format date
function formatDate(dateString: string): string {
  return new Date(dateString).toLocaleString('zh-CN');
}

// Get strategy type text
function getStrategyTypeText(type: string): string {
  const typeMap: Record<string, string> = {
    'TrendFollowing': '趋势跟踪',
    'MeanReversion': '均值回归',
    'Arbitrage': '套利',
    'MarketMaking': '做市',
    'Statistical': '统计套利',
    'MachineLearning': '机器学习',
    'Custom': '自定义'
  };
  return typeMap[type] || type;
}

// Fetch strategies
async function fetchStrategies() {
  loading.value = true;
  try {
    const data = await getStrategies();
    strategies.value = data;
  } catch (error) {
    console.error('Failed to fetch strategies:', error);
    ElMessage.error('获取策略列表失败');
  } finally {
    loading.value = false;
  }
}

// Open strategy dialog
function openStrategyDialog(strategy?: any) {
  if (strategy) {
    isEditing.value = true;
    currentStrategy.value = { ...strategy };
    // Parse strategy params
    strategyParams.value = strategy.params ? JSON.parse(JSON.stringify(strategy.params)) : {};
  } else {
    isEditing.value = false;
    currentStrategy.value = {
      strategy_id: '',
      strategy_name: '',
      strategy_type: 'TrendFollowing',
      params: {},
      enabled: true,
      max_position: 100000,
      max_daily_loss: 5000,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    strategyParams.value = {};
  }
  dialogVisible.value = true;
}

// Add parameter
function addParam() {
  const paramName = prompt('请输入参数名称:');
  if (paramName) {
    strategyParams.value[paramName] = '';
  }
}

// Save strategy
async function saveStrategy() {
  if (!strategyFormRef.value) return;
  
  await strategyFormRef.value.validate(async (valid) => {
    if (!valid) return;
    
    try {
      // Merge strategy params
      currentStrategy.value.params = strategyParams.value;
      
      await apiSaveStrategy(currentStrategy.value);
      ElMessage.success('策略保存成功');
      dialogVisible.value = false;
      fetchStrategies();
    } catch (error) {
      console.error('Failed to save strategy:', error);
      ElMessage.error('保存策略失败');
    }
  });
}

// Delete strategy — show ConfirmDialog first
function deleteStrategy(strategyId: string) {
  strategyToDelete.value = strategyId;
  deleteDialogVisible.value = true;
}

async function confirmDeleteStrategy() {
  if (!strategyToDelete.value) return;
  try {
    await apiDeleteStrategy(strategyToDelete.value);
    ElMessage.success('策略删除成功');
    deleteDialogVisible.value = false;
    strategyToDelete.value = null;
    fetchStrategies();
  } catch (error) {
    console.error('Failed to delete strategy:', error);
    ElMessage.error('删除策略失败');
  }
}

// Toggle strategy status
async function toggleStrategyStatus(strategy: any) {
  try {
    await toggleStrategy(strategy.strategy_id, strategy.enabled);
    ElMessage.success('策略状态更新成功');
  } catch (error) {
    console.error('Failed to toggle strategy status:', error);
    ElMessage.error('更新策略状态失败');
    // Revert the change
    strategy.enabled = !strategy.enabled;
  }
}

// Strategy lifecycle management
async function handleLifecycle(action: string, strategy: any) {
  const actionLabels: Record<string, string> = {
    deploy: '部署', start: '启动', stop: '停止',
    pause: '暂停', resume: '恢复', archive: '归档',
  }
  const label = actionLabels[action] || action
  try {
    const apiMap: Record<string, (id: string) => Promise<string>> = {
      deploy: deployStrategy,
      start: startStrategy,
      stop: stopStrategy,
      pause: pauseStrategy,
      resume: resumeStrategy,
      archive: archiveStrategy,
    }
    const api = apiMap[action]
    if (!api) { ElMessage.error('未知操作'); return }
    await api(strategy.strategy_id)
    ElMessage.success(`策略${label}成功`)
    fetchStrategies()
  } catch (error) {
    console.error(`Failed to ${action} strategy:`, error)
    ElMessage.error(`策略${label}失败: ${(error as Error).message}`)
  }
}

// Run backtest
async function runBacktest(strategyId: string) {
  try {
    loading.value = true;
    const now = new Date();
    const startDate = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
    const result = await apiRunBacktest(
      strategyId,
      startDate.toISOString(),
      now.toISOString(),
      1000000,  // initial capital
      0.0003,   // commission rate
      0.0001,   // slippage
      []        // symbols
    );
    backtestResult.value = result;
    backtestDialogVisible.value = true;
    setTimeout(() => {
      initBacktestChart();
    }, 100);
  } catch (error) {
    console.error('Failed to run backtest:', error);
    ElMessage.error('回测失败');
  } finally {
    loading.value = false;
  }
}

// Initialize backtest chart
function initBacktestChart() {
  const chartDom = document.getElementById('backtest-chart');
  if (!chartDom || !backtestResult.value) return;
  const chart = echarts.init(chartDom);
  
  const equity = backtestResult.value.equity_curve;
  let dates: string[];
  let values: number[];

  if (equity && equity.length > 0) {
    // Use real data from API
    dates = equity.map(([date]: [string, number]) => new Date(date).toLocaleDateString('zh-CN'));
    values = equity.map(([, val]: [string, number]) => val);
  } else {
    // No data to display
    chart.dispose();
    return;
  }
  
  const option = {
    tooltip: {
      trigger: 'axis',
      formatter: function(params: any) {
        return `${params[0].axisValue}<br/>¥${formatCurrency(params[0].value)}`;
      }
    },
    xAxis: {
      type: 'category',
      data: dates
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        formatter: function(value: number) {
          return '¥' + (value / 10000).toFixed(0) + '万';
        }
      }
    },
    series: [{
      data: values,
      type: 'line',
      smooth: true,
      areaStyle: {},
      lineStyle: { width: 2 },
      itemStyle: { color: '#409EFF' }
    }]
  };
  
  chart.setOption(option);
}

// Initialize on mount
onMounted(() => {
  fetchStrategies();
});

// Watch for backtest dialog visibility
watch(backtestDialogVisible, (newVal) => {
  if (newVal) {
    // Initialize chart when dialog opens
    setTimeout(() => {
      initBacktestChart();
    }, 100);
  }
});
</script>

<style scoped>
.strategy-management {
  padding: 20px;
}

.header {
  margin-bottom: 20px;
  align-items: center;
}

.controls {
  text-align: right;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.strategy-list-card {
  margin-bottom: 20px;
}

.table-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
  gap: 12px;
}

.batch-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.table-footer {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.card-header-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.strategy-params {
  width: 100%;
}

.param-item {
  margin-bottom: 10px;
}

.backtest-stat-card {
  margin-bottom: 20px;
}

.stat-item {
  text-align: center;
}

.stat-label {
  font-size: 14px;
  color: #999;
  margin-bottom: 8px;
}

.stat-value {
  font-size: 20px;
  font-weight: bold;
  color: #333;
}

.stat-value.positive {
  color: #67C23A;
}

.stat-value.negative {
  color: #F56C6C;
}

.dialog-footer {
  text-align: right;
}
</style>