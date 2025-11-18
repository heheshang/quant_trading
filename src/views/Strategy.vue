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
        </div>
      </template>
      
      <el-table :data="strategies" style="width: 100%" v-loading="loading">
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
        <el-table-column label="操作" width="200">
          <template #default="scope">
            <el-button size="small" @click="openStrategyDialog(scope.row)">编辑</el-button>
            <el-button size="small" type="primary" @click="runBacktest(scope.row.strategy_id)">回测</el-button>
            <el-button size="small" type="danger" @click="deleteStrategy(scope.row.strategy_id)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 策略编辑对话框 -->
    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="600px">
      <el-form :model="currentStrategy" label-width="120px">
        <el-form-item label="策略名称">
          <el-input v-model="currentStrategy.strategy_name" />
        </el-form-item>
        
        <el-form-item label="策略类型">
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
        
        <el-form-item label="最大持仓">
          <el-input-number v-model="currentStrategy.max_position" :min="0" :step="10000" />
        </el-form-item>
        
        <el-form-item label="最大日亏损">
          <el-input-number v-model="currentStrategy.max_daily_loss" :min="0" :step="1000" />
        </el-form-item>
        
        <el-form-item label="启用状态">
          <el-switch v-model="currentStrategy.enabled" />
        </el-form-item>
        
        <!-- 策略参数配置 -->
        <el-form-item label="策略参数">
          <div class="strategy-params">
            <div v-for="(value, key) in strategyParams" :key="key" class="param-item">
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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import * as echarts from 'echarts';
import { ElMessage, ElMessageBox } from 'element-plus';

// Reactive data
const loading = ref(false);
const strategies = ref<any[]>([]);
const dialogVisible = ref(false);
const backtestDialogVisible = ref(false);
const currentStrategy = ref<any>({});
const strategyParams = ref<Record<string, any>>({});
const backtestResult = ref<any>(null);
const isEditing = ref(false);

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
    const data = await invoke<any[]>('get_strategies');
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
  try {
    // Merge strategy params
    currentStrategy.value.params = strategyParams.value;
    
    await invoke<string>('save_strategy', { strategy: currentStrategy.value });
    ElMessage.success('策略保存成功');
    dialogVisible.value = false;
    fetchStrategies();
  } catch (error) {
    console.error('Failed to save strategy:', error);
    ElMessage.error('保存策略失败');
  }
}

// Delete strategy
async function deleteStrategy(strategyId: string) {
  try {
    await ElMessageBox.confirm('确定要删除这个策略吗？', '确认删除', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    });
    
    await invoke<boolean>('delete_strategy', { strategyId });
    ElMessage.success('策略删除成功');
    fetchStrategies();
  } catch (error) {
    if (error !== 'cancel') {
      console.error('Failed to delete strategy:', error);
      ElMessage.error('删除策略失败');
    }
  }
}

// Toggle strategy status
async function toggleStrategyStatus(strategy: any) {
  try {
    await invoke<boolean>('toggle_strategy', { 
      strategyId: strategy.strategy_id, 
      enabled: strategy.enabled 
    });
    ElMessage.success('策略状态更新成功');
  } catch (error) {
    console.error('Failed to toggle strategy status:', error);
    ElMessage.error('更新策略状态失败');
    // Revert the change
    strategy.enabled = !strategy.enabled;
  }
}

// Run backtest
async function runBacktest(strategyId: string) {
  try {
    loading.value = true;
    // Mock backtest result for now
    backtestResult.value = {
      strategy_id: strategyId,
      start_date: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
      end_date: new Date().toISOString(),
      initial_capital: 1000000,
      final_capital: 1120000,
      total_return: 0.12,
      annual_return: 0.48,
      sharpe_ratio: 1.8,
      max_drawdown: -0.08,
      win_rate: 0.65,
      profit_loss_ratio: 2.1,
      total_trades: 150,
      winning_trades: 98,
      losing_trades: 52,
      equity_curve: []
    };
    
    backtestDialogVisible.value = true;
    // Initialize backtest chart after dialog is visible
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
  if (chartDom && backtestResult.value) {
    const chart = echarts.init(chartDom);
    
    // Generate mock equity curve data
    const dates = [];
    const values = [];
    const startDate = new Date(backtestResult.value.start_date);
    const endDate = new Date(backtestResult.value.end_date);
    const days = Math.ceil((endDate.getTime() - startDate.getTime()) / (1000 * 60 * 60 * 24));
    
    let currentValue = backtestResult.value.initial_capital;
    for (let i = 0; i <= days; i++) {
      const date = new Date(startDate);
      date.setDate(date.getDate() + i);
      dates.push(date.toLocaleDateString('zh-CN'));
      
      // Simulate some random growth
      const dailyReturn = (Math.random() - 0.5) * 0.02;
      currentValue = currentValue * (1 + dailyReturn);
      values.push(currentValue);
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