<template>
  <div class="backtest-system">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>回测系统</h2>
      </el-col>
    </el-row>

    <!-- 回测配置 -->
    <el-card class="backtest-config-card">
      <template #header>
        <div class="card-header">
          <span>回测配置</span>
        </div>
      </template>
      
      <el-form :model="backtestConfig" :rules="backtestRules" ref="backtestFormRef" label-width="120px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="选择策略" prop="strategyId">
              <el-select v-model="backtestConfig.strategyId" placeholder="请选择策略" @change="onStrategyChange">
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
              <el-input v-model="backtestConfig.strategyName" readonly />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="开始日期" prop="startDate">
              <el-date-picker
                v-model="backtestConfig.startDate"
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
                v-model="backtestConfig.endDate"
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
              <el-input-number v-model="backtestConfig.initialCapital" :min="10000" :step="100000" style="width: 100%" />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="手续费率" prop="commissionRate">
              <el-input-number v-model="backtestConfig.commissionRate" :min="0" :max="0.1" :step="0.001" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="滑点" prop="slippage">
              <el-input-number v-model="backtestConfig.slippage" :min="0" :max="0.1" :step="0.001" style="width: 100%" />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="标的代码" prop="symbols">
              <el-input v-model="backtestConfig.symbols" placeholder="多个标的用逗号分隔" />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-form-item>
          <el-button type="primary" @click="runBacktest" :loading="running">开始回测</el-button>
          <el-button @click="saveTemplate">保存模板</el-button>
          <el-dropdown @command="loadTemplate" v-if="templates.length > 0">
            <el-button>加载模板<el-icon class="el-icon--right"><ArrowDown /></el-icon></el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item v-for="(tpl, i) in templates" :key="i" :command="i">
                  {{ tpl.name }}
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <el-button @click="resetConfig">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 回测结果 -->
    <el-card class="backtest-result-card" v-if="backtestResult">
      <template #header>
        <div class="card-header">
          <span>回测结果</span>
          <el-button @click="exportResult">导出结果</el-button>
        </div>
      </template>
      
      <el-tabs v-model="activeTab">
        <!-- 概览 -->
        <el-tab-pane label="概览" name="overview">
          <el-row :gutter="20">
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">总收益率</div>
                  <div class="stat-value" :class="{ positive: backtestResult.total_return > 0, negative: backtestResult.total_return < 0 }">
                    {{ formatPercentage(backtestResult.total_return) }}
                  </div>
                </div>
              </el-card>
            </el-col>
            
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">年化收益率</div>
                  <div class="stat-value" :class="{ positive: backtestResult.annual_return > 0, negative: backtestResult.annual_return < 0 }">
                    {{ formatPercentage(backtestResult.annual_return) }}
                  </div>
                </div>
              </el-card>
            </el-col>
            
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">夏普比率</div>
                  <div class="stat-value">{{ formatNumber(backtestResult.sharpe_ratio) }}</div>
                </div>
              </el-card>
            </el-col>
            
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">最大回撤</div>
                  <div class="stat-value negative">{{ formatPercentage(backtestResult.max_drawdown) }}</div>
                </div>
              </el-card>
            </el-col>
          </el-row>
          
          <el-row :gutter="20" style="margin-top: 20px;">
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">胜率</div>
                  <div class="stat-value">{{ formatPercentage(backtestResult.win_rate) }}</div>
                </div>
              </el-card>
            </el-col>
            
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">总交易数</div>
                  <div class="stat-value">{{ backtestResult.total_trades }}</div>
                </div>
              </el-card>
            </el-col>
            
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">初始资金</div>
                  <div class="stat-value">¥{{ formatCurrency(backtestResult.initial_capital) }}</div>
                </div>
              </el-card>
            </el-col>
            
            <el-col :span="6">
              <el-card class="backtest-stat-card">
                <div class="stat-item">
                  <div class="stat-label">最终资金</div>
                  <div class="stat-value">¥{{ formatCurrency(backtestResult.final_capital) }}</div>
                </div>
              </el-card>
            </el-col>
          </el-row>
        </el-tab-pane>
        
        <!-- 收益曲线 -->
        <el-tab-pane label="收益曲线" name="equity">
          <el-card>
            <div id="equity-chart" style="height: 400px;"></div>
          </el-card>
        </el-tab-pane>
        
        <!-- 交易记录 -->
        <el-tab-pane label="交易记录" name="trades">
          <el-table v-if="tradeRecords.length > 0" :data="tradeRecords" style="width: 100%">
            <el-table-column prop="date" label="日期" width="180" />
            <el-table-column prop="symbol" label="标的" width="120" />
            <el-table-column prop="type" label="类型" width="100" />
            <el-table-column prop="price" label="价格" width="120">
              <template #default="scope">
                ¥{{ formatCurrency(scope.row.price) }}
              </template>
            </el-table-column>
            <el-table-column prop="quantity" label="数量" width="100" />
            <el-table-column prop="amount" label="金额" width="120">
              <template #default="scope">
                ¥{{ formatCurrency(scope.row.amount) }}
              </template>
            </el-table-column>
            <el-table-column prop="commission" label="手续费" width="100">
              <template #default="scope">
                ¥{{ formatCurrency(scope.row.commission) }}
              </template>
            </el-table-column>
          </el-table>
          <EmptyState v-else title="暂无交易记录" description="回测完成后将在此处显示交易明细" />
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 历史记录 -->
    <el-card class="history-card">
      <template #header>
        <div class="card-header">
          <span>回测历史记录</span>
          <div class="card-header-controls">
            <el-button @click="compareMode = !compareMode" size="small" :type="compareMode ? 'primary' : 'default'">
              {{ compareMode ? '取消对比' : '对比' }}
            </el-button>
            <el-button @click="exportHistoryCSV" size="small">导出CSV</el-button>
            <el-button @click="fetchHistory" :loading="historyLoading" size="small">刷新</el-button>
          </div>
        </div>
      </template>
      
      <el-table v-if="historyRecords.length > 0" :data="historyRecords" style="width: 100%" v-loading="historyLoading" @selection-change="onHistorySelectionChange">
        <el-table-column v-if="compareMode" type="selection" width="50" />
        <el-table-column prop="strategy_name" label="策略名称" width="150">
          <template #default="scope">
            {{ scope.row.strategy_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="start_date" label="开始日期" width="120">
          <template #default="scope">
            {{ formatDate(scope.row.start_date) }}
          </template>
        </el-table-column>
        <el-table-column prop="end_date" label="结束日期" width="120">
          <template #default="scope">
            {{ formatDate(scope.row.end_date) }}
          </template>
        </el-table-column>
        <el-table-column prop="total_return" label="总收益率" width="100">
          <template #default="scope">
            <span :class="{ positive: scope.row.total_return > 0, negative: scope.row.total_return < 0 }">
              {{ formatPercentage(scope.row.total_return) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="sharpe_ratio" label="夏普比率" width="100">
          <template #default="scope">
            {{ formatNumber(scope.row.sharpe_ratio) }}
          </template>
        </el-table-column>
        <el-table-column prop="max_drawdown" label="最大回撤" width="100">
          <template #default="scope">
            <span class="negative">{{ formatPercentage(scope.row.max_drawdown) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="total_trades" label="交易数" width="80" />
        <el-table-column prop="win_rate" label="胜率" width="80">
          <template #default="scope">
            {{ formatPercentage(scope.row.win_rate) }}
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="160">
          <template #default="scope">
            {{ formatDate(scope.row.created_at) }}
          </template>
        </el-table-column>
        <el-table-column label="操作" width="160" fixed="right">
          <template #default="scope">
            <el-button size="small" @click="viewHistoryDetail(scope.row.id)">详情</el-button>
            <el-button size="small" type="danger" @click="deleteHistoryRecord(scope.row.id)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <EmptyState v-else-if="!historyLoading" title="暂无回测记录" description="请先运行回测来生成记录" />
    </el-card>

    <!-- 加载状态 -->
    <el-card class="loading-card" v-if="running">
      <div class="loading-content">
        <el-skeleton animated>
          <template #template>
            <el-skeleton-item variant="text" style="width: 30%" />
            <el-skeleton-item variant="text" style="width: 50%" />
            <el-skeleton-item variant="text" style="width: 70%" />
          </template>
        </el-skeleton>
        <div class="loading-text">正在执行回测，请稍候...</div>
      </div>
    </el-card>
  
  <!-- Comparison dialog -->
  <el-dialog v-model="compareDialogVisible" title="结果对比" width="900px">
    <el-row :gutter="20" v-if="compareResults.length === 2">
      <el-col :span="12" v-for="(r, i) in compareResults" :key="i">
        <el-card>
          <template #header><span>{{ r.strategy_name || `结果 ${i + 1}` }}</span></template>
          <div class="compare-stats">
            <div class="compare-row"><span class="cl">总收益率</span><span :class="{ positive: r.total_return > 0, negative: r.total_return < 0 }">{{ formatPercentage(r.total_return) }}</span></div>
            <div class="compare-row"><span class="cl">年化收益率</span><span :class="{ positive: r.annual_return > 0, negative: r.annual_return < 0 }">{{ formatPercentage(r.annual_return) }}</span></div>
            <div class="compare-row"><span class="cl">夏普比率</span><span>{{ formatNumber(r.sharpe_ratio) }}</span></div>
            <div class="compare-row"><span class="cl">最大回撤</span><span class="negative">{{ formatPercentage(r.max_drawdown) }}</span></div>
            <div class="compare-row"><span class="cl">胜率</span><span>{{ formatPercentage(r.win_rate) }}</span></div>
            <div class="compare-row"><span class="cl">总交易数</span><span>{{ r.total_trades }}</span></div>
            <div class="compare-row"><span class="cl">盈亏比</span><span>{{ formatNumber(r.profit_loss_ratio) }}</span></div>
          </div>
        </el-card>
      </el-col>
    </el-row>
    <p v-else style="text-align:center;color:#999;">请选择两条记录进行对比</p>
    <template #footer>
      <el-button @click="compareDialogVisible = false">关闭</el-button>
    </template>
  </el-dialog>

  <!-- Delete history record confirm dialog -->
  <ConfirmDialog
    v-model:visible="deleteDialogVisible"
    title="确认删除"
    message="确定要删除这条回测记录吗？此操作不可撤销。"
    type="danger"
    confirm-text="删除"
    @confirm="confirmDeleteRecord"
  />
</div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { getStrategies, runBacktest as apiRunBacktest, getBacktestResults, getBacktestResult, deleteBacktestResult } from '@/services/api';
import * as echarts from 'echarts';
import { ElMessage, FormInstance } from 'element-plus';
import { ArrowDown } from '@element-plus/icons-vue';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';
import EmptyState from '@/components/common/EmptyState.vue';

// Form reference
const backtestFormRef = ref<FormInstance>();

// Validation rules
const backtestRules = {
  strategyId: [
    { required: true, message: '请选择策略', trigger: 'change' }
  ],
  startDate: [
    { required: true, message: '请选择开始日期', trigger: 'change' }
  ],
  endDate: [
    { required: true, message: '请选择结束日期', trigger: 'change' }
  ],
  initialCapital: [
    { required: true, message: '请输入初始资金', trigger: 'blur' },
    { type: 'number', min: 10000, message: '初始资金不能少于10,000', trigger: 'blur' }
  ],
  commissionRate: [
    { required: true, message: '请输入手续费率', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '手续费率应在0-0.1之间', trigger: 'blur' }
  ],
  slippage: [
    { required: true, message: '请输入滑点', trigger: 'blur' },
    { type: 'number', min: 0, max: 0.1, message: '滑点应在0-0.1之间', trigger: 'blur' }
  ],
  symbols: [
    { required: true, message: '请输入标的代码', trigger: 'blur' }
  ]
};

// Reactive data
const strategies = ref<any[]>([]);
const backtestConfig = ref({
  strategyId: '',
  strategyName: '',
  startDate: '',
  endDate: '',
  initialCapital: 1000000,
  commissionRate: 0.001,
  slippage: 0.0005,
  symbols: 'BTC-USDT'
});
const backtestResult = ref<any>(null);
const tradeRecords = ref<any[]>([]);
const running = ref(false);
const activeTab = ref('overview');
const historyRecords = ref<any[]>([]);
const historyLoading = ref(false);
const deleteDialogVisible = ref(false);
const recordToDelete = ref<number | null>(null);

// ==================== Parameter templates ====================
const templates = ref<{ name: string; config: typeof backtestConfig.value }[]>([])

function saveTemplate() {
  const name = prompt('输入模板名称：')
  if (!name) return
  templates.value.push({ name, config: { ...backtestConfig.value } })
  ElMessage.success(`模板「${name}」已保存`)
}

function loadTemplate(index: number) {
  const tpl = templates.value[index]
  if (tpl) {
    backtestConfig.value = { ...tpl.config }
    ElMessage.success(`已加载模板「${tpl.name}」`)
  }
}

// ==================== History comparison ====================
const compareMode = ref(false)
const compareDialogVisible = ref(false)
const compareResults = ref<any[]>([])
const selectedHistoryIds = ref<number[]>([])

function onHistorySelectionChange(rows: any[]) {
  selectedHistoryIds.value = rows.map((r: any) => r.id)
  if (compareMode && selectedHistoryIds.value.length === 2) {
    compareResults.value = rows
    compareDialogVisible.value = true
  }
}

watch(compareMode, (val) => {
  if (!val) { selectedHistoryIds.value = []; compareResults.value = [] }
})

// ==================== CSV export ====================
function exportHistoryCSV() {
  const headers = ['策略名称', '开始日期', '结束日期', '总收益率', '夏普比率', '最大回撤', '交易数', '胜率', '创建时间']
  const rows = historyRecords.value.map((r: any) => [
    r.strategy_name, r.start_date, r.end_date,
    formatPercentage(r.total_return), formatNumber(r.sharpe_ratio),
    formatPercentage(r.max_drawdown), r.total_trades,
    formatPercentage(r.win_rate), r.created_at,
  ])
  const csv = [headers.join(','), ...rows.map((r: string[]) => r.join(','))].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url; a.download = `backtest_history_${new Date().toISOString().slice(0, 10)}.csv`
  a.click(); URL.revokeObjectURL(url)
}

function exportResult() {
  if (!backtestResult.value) return
  const headers = ['指标', '值']
  const rows = [
    ['总收益率', formatPercentage(backtestResult.value.total_return)],
    ['年化收益率', formatPercentage(backtestResult.value.annual_return)],
    ['夏普比率', formatNumber(backtestResult.value.sharpe_ratio)],
    ['最大回撤', formatPercentage(backtestResult.value.max_drawdown)],
    ['胜率', formatPercentage(backtestResult.value.win_rate)],
    ['总交易数', backtestResult.value.total_trades],
    ['初始资金', formatCurrency(backtestResult.value.initial_capital)],
    ['最终资金', formatCurrency(backtestResult.value.final_capital)],
  ]
  const csv = [headers.join(','), ...rows.map((r: string[]) => r.join(','))].join('\n')
  const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url; a.download = `backtest_result_${new Date().toISOString().slice(0, 10)}.csv`
  a.click(); URL.revokeObjectURL(url)
}

// Format currency
function formatCurrency(value: any): string {
  if (value === null || value === undefined) return '0.00';
  return parseFloat(value.toString()).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

// Format percentage
function formatPercentage(value: any): string {
  if (value === null || value === undefined) return '0.00%';
  return (parseFloat(value.toString()) * 100).toFixed(2) + '%';
}

// Format number (handles null/undefined gracefully)
function formatNumber(value: any): string {
  if (value === null || value === undefined) return '-';
  return parseFloat(value.toString()).toFixed(2);
}

// Format date string for display
function formatDate(dateStr: string): string {
  if (!dateStr) return '-';
  return new Date(dateStr).toLocaleString('zh-CN');
}

// Fetch strategies
async function fetchStrategies() {
  try {
    strategies.value = await getStrategies() as any;
  } catch (error) {
    console.error('Failed to fetch strategies:', error);
    ElMessage.error('获取策略列表失败');
  }
}

// Strategy change handler
function onStrategyChange(strategyId: string) {
  const strategy = strategies.value.find(s => s.strategy_id === strategyId);
  if (strategy) {
    backtestConfig.value.strategyName = strategy.strategy_name;
  }
}

// Reset config
function resetConfig() {
  backtestConfig.value = {
    strategyId: '',
    strategyName: '',
    startDate: '',
    endDate: '',
    initialCapital: 1000000,
    commissionRate: 0.001,
    slippage: 0.0005,
    symbols: 'BTC-USDT'
  };
  backtestResult.value = null;
  tradeRecords.value = [];
}

// Fetch history records
async function fetchHistory() {
  historyLoading.value = true;
  try {
    const data = await getBacktestResults(50, 0);
    historyRecords.value = data as any;
  } catch (error) {
    console.error('Failed to fetch backtest history:', error);
  } finally {
    historyLoading.value = false;
  }
}

// View history detail
async function viewHistoryDetail(id: number) {
  try {
    backtestResult.value = await getBacktestResult(id) as any;
    activeTab.value = 'overview';
  } catch (error) {
    console.error('Failed to fetch backtest detail:', error);
    ElMessage.error('获取回测详情失败');
  }
}

// Delete history record — show ConfirmDialog first
function deleteHistoryRecord(id: number) {
  recordToDelete.value = id;
  deleteDialogVisible.value = true;
}

async function confirmDeleteRecord() {
  const id = recordToDelete.value;
  if (id === null) return;
  try {
    await deleteBacktestResult(id);
    ElMessage.success('删除成功');
    deleteDialogVisible.value = false;
    recordToDelete.value = null;
    fetchHistory();
  } catch (error) {
    console.error('Failed to delete backtest result:', error);
    ElMessage.error('删除失败');
  }
}

// Run backtest
async function runBacktest() {
  // Element Plus form validation first
  if (!backtestFormRef.value) return;
  try {
    await backtestFormRef.value.validate();
  } catch {
    // Validation failed — Element Plus already shows error messages on fields
    return;
  }

  // Manual validation checks (extra safety)
  if (!backtestConfig.value.strategyId) {
    ElMessage.warning('请选择策略');
    return;
  }
  
  if (!backtestConfig.value.startDate || !backtestConfig.value.endDate) {
    ElMessage.warning('请选择回测时间范围');
    return;
  }
  
  running.value = true;
  try {
    const symbols = backtestConfig.value.symbols
      .split(',')
      .map(s => s.trim())
      .filter(s => s.length > 0);

    const result = await apiRunBacktest(
      backtestConfig.value.strategyId,
      backtestConfig.value.startDate,
      backtestConfig.value.endDate,
      backtestConfig.value.initialCapital,
      backtestConfig.value.commissionRate,
      backtestConfig.value.slippage,
      symbols,
    );
    
    backtestResult.value = result;
    
    // Use real trade records from API if available, otherwise empty
    tradeRecords.value = (result as any).trades || [];
    
    ElMessage.success('回测完成');
    fetchHistory();
  } catch (error) {
    console.error('Backtest failed:', error);
    ElMessage.error('回测失败: ' + (error as Error).message);
  } finally {
    running.value = false;
  }
}

// Initialize equity chart
function initEquityChart() {
  const chartDom = document.getElementById('equity-chart');
  if (chartDom && backtestResult.value) {
    const chart = echarts.getInstanceByDom(chartDom) || echarts.init(chartDom);
    
    // Process equity curve data
    const dates = backtestResult.value.equity_curve.map((item: any) => 
      new Date(item[0]).toLocaleDateString('zh-CN')
    );
    const values = backtestResult.value.equity_curve.map((item: any) => 
      parseFloat(item[1].toString())
    );
    
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
}

// Initialize on mount
onMounted(() => {
  fetchStrategies();
  fetchHistory();
  
  // Set default dates
  const today = new Date();
  const oneMonthAgo = new Date();
  oneMonthAgo.setMonth(oneMonthAgo.getMonth() - 1);
  
  backtestConfig.value.startDate = oneMonthAgo.toISOString().split('T')[0];
  backtestConfig.value.endDate = today.toISOString().split('T')[0];
});

// Watch for backtest result changes
watch(backtestResult, (newVal) => {
  if (newVal) {
    // Initialize chart after result is available
    setTimeout(() => {
      initEquityChart();
    }, 100);
  }
});

// Watch for active tab changes
watch(activeTab, (newTab) => {
  if (newTab === 'equity' && backtestResult.value) {
    // Initialize chart when switching to equity tab
    setTimeout(() => {
      initEquityChart();
    }, 100);
  }
});
</script>

<style scoped>
.backtest-system {
  padding: 20px;
}

.header {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.backtest-config-card {
  margin-bottom: 20px;
}

.backtest-result-card {
  margin-bottom: 20px;
}

.backtest-stat-card {
  margin-bottom: 20px;
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
  color: #606266;
}

.compare-row .positive { color: #67c23a; }
.compare-row .negative { color: #f56c6c; }

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

.loading-card {
  text-align: center;
}

.loading-content {
  padding: 40px 20px;
}

.loading-text {
  margin-top: 20px;
  color: #999;
}
</style>