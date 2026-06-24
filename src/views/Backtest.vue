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
                  <div class="stat-value">{{ backtestResult.sharpe_ratio.toFixed(2) }}</div>
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
          <el-table :data="tradeRecords" style="width: 100%">
            <el-table-column prop="date" label="日期" width="180" />
            <el-table-column prop="symbol" label="标的" width="120" />
            <el-table-column prop="type" label="类型" width="100" />
            <el-table-column prop="price" label="价格" width="120">
              <template #default="scope">
                ¥{{ scope.row.price.toFixed(2) }}
              </template>
            </el-table-column>
            <el-table-column prop="quantity" label="数量" width="100" />
            <el-table-column prop="amount" label="金额" width="120">
              <template #default="scope">
                ¥{{ scope.row.amount.toFixed(2) }}
              </template>
            </el-table-column>
            <el-table-column prop="commission" label="手续费" width="100">
              <template #default="scope">
                ¥{{ scope.row.commission.toFixed(2) }}
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import * as echarts from 'echarts';
import { ElMessage, FormInstance } from 'element-plus';

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
  symbols: '600519.SH'
});
const backtestResult = ref<any>(null);
const tradeRecords = ref<any[]>([]);
const running = ref(false);
const activeTab = ref('overview');

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

// Fetch strategies
async function fetchStrategies() {
  try {
    const data = await invoke<any[]>('get_strategies');
    strategies.value = data;
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
    symbols: '600519.SH'
  };
  backtestResult.value = null;
  tradeRecords.value = [];
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
    const result = await invoke<any>('run_backtest', {
      strategyId: backtestConfig.value.strategyId,
      startDate: backtestConfig.value.startDate,
      endDate: backtestConfig.value.endDate
    });
    
    backtestResult.value = result;
    
    // Generate mock trade records for display
    tradeRecords.value = [
      {
        date: new Date().toISOString(),
        symbol: '600519.SH',
        type: '买入',
        price: 1650.00,
        quantity: 100,
        amount: 165000,
        commission: 165
      },
      {
        date: new Date(Date.now() + 86400000).toISOString(),
        symbol: '600519.SH',
        type: '卖出',
        price: 1680.00,
        quantity: 100,
        amount: 168000,
        commission: 168
      }
    ];
    
    ElMessage.success('回测完成');
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

// Export result
function exportResult() {
  ElMessage.info('导出功能开发中...');
}

// Initialize on mount
onMounted(() => {
  fetchStrategies();
  
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