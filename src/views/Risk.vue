<template>
  <div class="risk-management">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>风险管理</h2>
      </el-col>
    </el-row>

    <!-- 风险指标 -->
    <el-card class="risk-metrics-card">
      <template #header>
        <div class="card-header">
          <span>风险指标</span>
          <el-button @click="refreshMetrics">刷新</el-button>
        </div>
      </template>
      
      <el-row :gutter="20">
        <el-col :span="6">
          <el-card class="risk-stat-card">
            <div class="stat-item">
              <div class="stat-label">VaR (95%)</div>
              <div class="stat-value">¥{{ formatCurrency(riskMetrics.var_95 * 1000000) }}</div>
            </div>
          </el-card>
        </el-col>
        
        <el-col :span="6">
          <el-card class="risk-stat-card">
            <div class="stat-item">
              <div class="stat-label">VaR (99%)</div>
              <div class="stat-value">¥{{ formatCurrency(riskMetrics.var_99 * 1000000) }}</div>
            </div>
          </el-card>
        </el-col>
        
        <el-col :span="6">
          <el-card class="risk-stat-card">
            <div class="stat-item">
              <div class="stat-label">最大持仓比例</div>
              <div class="stat-value">{{ (riskMetrics.max_position_size * 100).toFixed(1) }}%</div>
            </div>
          </el-card>
        </el-col>
        
        <el-col :span="6">
          <el-card class="risk-stat-card">
            <div class="stat-item">
              <div class="stat-label">单日最大亏损</div>
              <div class="stat-value">{{ (riskMetrics.max_daily_loss * 100).toFixed(1) }}%</div>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </el-card>

    <!-- 风险指标趋势图 -->
    <el-card class="risk-chart-card" style="margin-top: 20px;">
      <template #header>
        <div class="card-header">
          <span>风险指标趋势</span>
        </div>
      </template>
      <div id="risk-trend-chart" style="height: 300px;"></div>
    </el-card>

    <!-- 风险配置 -->
    <el-card class="risk-config-card" style="margin-top: 20px;">
      <template #header>
        <div class="card-header">
          <span>风险配置</span>
          <el-button type="primary" @click="saveConfig" :loading="saving">保存配置</el-button>
        </div>
      </template>
      
      <el-form ref="riskConfigFormRef" :model="riskConfig" :rules="riskConfigRules" label-width="150px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="最大持仓比例" prop="max_position_size">
              <el-slider 
                v-model="riskConfig.max_position_size" 
                :min="0" 
                :max="1" 
                :step="0.01" 
                show-input
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="单日最大亏损比例" prop="max_daily_loss">
              <el-slider 
                v-model="riskConfig.max_daily_loss" 
                :min="0" 
                :max="0.2" 
                :step="0.001" 
                show-input
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="最大回撤限制" prop="max_drawdown">
              <el-slider 
                v-model="riskConfig.max_drawdown" 
                :min="0" 
                :max="0.3" 
                :step="0.01" 
                show-input
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="VaR置信水平" prop="var_confidence_level">
              <el-slider 
                v-model="riskConfig.var_confidence_level" 
                :min="0.9" 
                :max="0.999" 
                :step="0.001" 
                show-input
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="启用事前检查" prop="enable_pre_trade_check">
              <el-switch v-model="riskConfig.enable_pre_trade_check" />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="启用实时监控" prop="enable_real_time_monitor">
              <el-switch v-model="riskConfig.enable_real_time_monitor" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
    </el-card>

    <!-- 事前风控测试 -->
    <el-card class="pre-trade-check-card">
      <template #header>
        <div class="card-header">
          <span>事前风控测试</span>
        </div>
      </template>
      
      <el-form ref="testOrderFormRef" :model="testOrder" :rules="testOrderRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="标的代码" prop="symbol">
              <el-input v-model="testOrder.symbol" placeholder="输入标的代码" />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="买卖方向" prop="side">
              <el-select v-model="testOrder.side" placeholder="选择方向" style="width: 100%">
                <el-option label="买入" value="Buy" />
                <el-option label="卖出" value="Sell" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="价格" prop="price">
              <el-input-number 
                v-model="testOrder.price" 
                :min="0" 
                :precision="2" 
                :step="0.01" 
                style="width: 100%" 
              />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="数量" prop="quantity">
              <el-input-number 
                v-model="testOrder.quantity" 
                :min="0" 
                :precision="2" 
                :step="100" 
                style="width: 100%" 
              />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-form-item>
          <el-button type="primary" @click="runPreTradeCheck" :loading="checking">风控检查</el-button>
          <el-button @click="resetTestOrder">重置</el-button>
        </el-form-item>
      </el-form>
      
      <el-alert 
        v-if="checkResult !== null" 
        :type="checkResult ? 'success' : 'error'" 
        :title="checkResult ? '风控检查通过' : '风控检查未通过'" 
        :closable="false"
        show-icon
      />
    </el-card>

    <!-- 风险告警 -->
    <el-card class="risk-alerts-card">
      <template #header>
        <div class="card-header">
          <span>风险告警</span>
          <div style="display:flex;gap:8px;align-items:center">
            <el-select v-model="alertLevelFilter" placeholder="级别筛选" size="small" clearable style="width:120px">
              <el-option label="严重" value="Critical" />
              <el-option label="警告" value="Warning" />
              <el-option label="信息" value="Info" />
            </el-select>
            <el-button @click="refreshAlerts">刷新</el-button>
          </div>
        </div>
      </template>
      
      <el-table v-if="filteredAlerts.length > 0" :data="filteredAlerts" style="width: 100%">
        <el-table-column prop="level" label="级别" width="80">
          <template #default="scope">
            <el-tag :type="getAlertLevelType(scope.row.level)">
              {{ getAlertLevelText(scope.row.level) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="source" label="来源" width="120" />
        <el-table-column prop="message" label="消息" />
        <el-table-column prop="timestamp" label="时间" width="180">
          <template #default="scope">
            {{ formatDate(scope.row.timestamp) }}
          </template>
        </el-table-column>
        <el-table-column label="操作" width="100">
          <template #default="scope">
            <el-button 
              size="small" 
              type="primary" 
              @click="acknowledgeAlert(scope.row.alert_id)"
              :disabled="scope.row.acknowledged"
            >
              {{ scope.row.acknowledged ? '已确认' : '确认' }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <EmptyState v-else title="暂无告警" description="当前没有风控告警" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick } from 'vue';
import * as echarts from 'echarts';
import { getAlerts, acknowledgeAlert as apiAcknowledgeAlert, preTradeCheck, updateRiskConfig, getRiskMetrics, getRiskConfig } from '@/services/api';
import { ElMessage, FormInstance } from 'element-plus';
import EmptyState from '@/components/common/EmptyState.vue';

// Reactive data
const riskMetrics = ref({
  var_95: 0.02,
  var_99: 0.04,
  max_position_size: 0.2,
  max_daily_loss: 0.05,
  max_drawdown: 0.15
});

const riskConfig = ref({
  max_position_size: 0.2,
  max_daily_loss: 0.05,
  max_drawdown: 0.15,
  enable_pre_trade_check: true,
  enable_real_time_monitor: true,
  var_confidence_level: 0.95
});

const riskAlerts = ref<any[]>([]);

const testOrder = ref({
  order_id: 0,
  strategy_id: 'test_strategy',
  symbol: '600519.SH',
  order_type: 'Limit',
  side: 'Buy',
  price: 1685.00,
  quantity: 100,
  filled_quantity: 0,
  status: 'Pending',
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  commission: 0,
  slippage: 0
});

const riskConfigFormRef = ref<FormInstance>();
const testOrderFormRef = ref<FormInstance>();
const checkResult = ref<boolean | null>(null);
const saving = ref(false);
const checking = ref(false);

// Alert filtering
const alertLevelFilter = ref('')
const filteredAlerts = computed(() => {
  if (!alertLevelFilter.value) return riskAlerts.value
  return riskAlerts.value.filter((a: any) => a.level === alertLevelFilter.value)
})

// Validation rules
const riskConfigRules = {
  max_position_size: [
    { required: true, message: '请设置最大持仓比例', trigger: 'change' },
    { type: 'number', min: 0, max: 1, message: '最大持仓比例应在0-1之间', trigger: 'change' }
  ],
  max_daily_loss: [
    { required: true, message: '请设置单日最大亏损比例', trigger: 'change' },
    { type: 'number', min: 0, max: 0.2, message: '单日最大亏损比例应在0-0.2之间', trigger: 'change' }
  ],
  max_drawdown: [
    { required: true, message: '请设置最大回撤限制', trigger: 'change' },
    { type: 'number', min: 0, max: 0.3, message: '最大回撤限制应在0-0.3之间', trigger: 'change' }
  ],
  var_confidence_level: [
    { required: true, message: '请设置VaR置信水平', trigger: 'change' },
    { type: 'number', min: 0.9, max: 0.999, message: 'VaR置信水平应在0.9-0.999之间', trigger: 'change' }
  ]
};

const testOrderRules = {
  symbol: [
    { required: true, message: '请输入标的代码', trigger: 'blur' }
  ],
  side: [
    { required: true, message: '请选择买卖方向', trigger: 'change' }
  ],
  price: [
    { required: true, message: '请输入价格', trigger: 'blur' },
    { type: 'number', min: 0, message: '价格不能为负数', trigger: 'blur' }
  ],
  quantity: [
    { required: true, message: '请输入数量', trigger: 'blur' },
    { type: 'number', min: 0, message: '数量不能为负数', trigger: 'blur' }
  ]
};

// Format currency
function formatCurrency(value: number): string {
  return value.toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

// Format date
function formatDate(date: string): string {
  return new Date(date).toLocaleString('zh-CN');
}

// Get alert level type for tag
function getAlertLevelType(level: string): string {
  switch (level) {
    case 'Info':
      return '';
    case 'Warning':
      return 'warning';
    case 'Critical':
      return 'danger';
    default:
      return 'info';
  }
}

// Get alert level text
function getAlertLevelText(level: string): string {
  switch (level) {
    case 'Info':
      return '信息';
    case 'Warning':
      return '警告';
    case 'Critical':
      return '严重';
    default:
      return level;
  }
}

// Generate a simple ID using timestamp
function generateId(): number {
  return Date.now();
}

// Fetch risk metrics
async function fetchRiskMetrics() {
  try {
    riskMetrics.value = await getRiskMetrics() as any;
  } catch (error) {
    console.error('Failed to fetch risk metrics:', error);
    ElMessage.error('获取风险指标失败');
  }
}

// Fetch risk config
async function fetchRiskConfig() {
  try {
    riskConfig.value = await getRiskConfig() as any;
  } catch (error) {
    console.error('Failed to fetch risk config:', error);
    ElMessage.error('获取风险配置失败');
  }
}

// Fetch risk alerts
async function fetchRiskAlerts() {
  try {
    riskAlerts.value = await getAlerts() as any;
  } catch (error) {
    console.error('Failed to fetch risk alerts:', error);
    ElMessage.error('获取风险告警失败');
  }
}

// Save config
async function saveConfig() {
  if (!riskConfigFormRef.value) return;

  await riskConfigFormRef.value.validate(async (valid) => {
    if (!valid) return;

    saving.value = true;
    try {
      await updateRiskConfig(riskConfig.value as any);
      ElMessage.success('风险配置保存成功');
    } catch (error) {
      console.error('Failed to save risk config:', error);
      ElMessage.error('保存风险配置失败: ' + (error as Error).message);
    } finally {
      saving.value = false;
    }
  });
}

// Run pre-trade check
async function runPreTradeCheck() {
  if (!testOrderFormRef.value) return;

  await testOrderFormRef.value.validate(async (valid) => {
    if (!valid) return;

    checking.value = true;
    try {
      // Generate a new order ID
      testOrder.value.order_id = generateId();
      
      // Mock account and positions data for testing
      const account = {
        account_id: 0,
        total_assets: 1000000,
        available_cash: 500000,
        frozen_cash: 0,
        market_value: 500000,
        total_pnl: 10000,
        daily_pnl: 5000,
        margin: 0,
        margin_ratio: 0,
        updated_at: new Date().toISOString()
      };
      
      const positions = [
        {
          symbol: "600519.SH",
          quantity: 1000,
          available_quantity: 1000,
          avg_price: 1650.00,
          market_value: 1685000,
          unrealized_pnl: 35000,
          realized_pnl: 0,
          updated_at: new Date().toISOString()
        }
      ];
      
      const result = await preTradeCheck(
        testOrder.value as any,
        account as any,
        positions
      );
      
      checkResult.value = result;
      ElMessage.success(result ? '风控检查通过' : '风控检查未通过');
    } catch (error) {
      console.error('Failed to run pre-trade check:', error);
      ElMessage.error('风控检查失败: ' + (error as Error).message);
    } finally {
      checking.value = false;
    }
  });
}

// Acknowledge alert
async function acknowledgeAlert(alertId: number) {
  try {
    await apiAcknowledgeAlert(alertId);
    ElMessage.success('告警确认成功');
    await fetchRiskAlerts();
  } catch (error) {
    console.error('Failed to acknowledge alert:', error);
    ElMessage.error('告警确认失败: ' + (error as Error).message);
  }
}

// Reset test order
function resetTestOrder() {
  testOrder.value = {
    order_id: 0,
    strategy_id: 'test_strategy',
    symbol: '600519.SH',
    order_type: 'Limit',
    side: 'Buy',
    price: 1685.00,
    quantity: 100,
    filled_quantity: 0,
    status: 'Pending',
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    commission: 0,
    slippage: 0
  };
  checkResult.value = null;
}

// Refresh metrics
async function refreshMetrics() {
  await fetchRiskMetrics();
  ElMessage.success('刷新成功');
}

// Refresh alerts
async function refreshAlerts() {
  await fetchRiskAlerts();
  ElMessage.success('刷新成功');
}

// Initialize risk trend chart
function initRiskChart() {
  const dom = document.getElementById('risk-trend-chart')
  if (!dom) return
  const chart = echarts.init(dom)
  const now = Date.now()
  const dates: string[] = []
  const var95: number[] = []
  const var99: number[] = []
  const dd: number[] = []
  for (let i = 29; i >= 0; i--) {
    const d = new Date(now - i * 86400000)
    dates.push(d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' }))
    var95.push(riskMetrics.value.var_95 * (1 + Math.sin(i / 7) * 0.3))
    var99.push(riskMetrics.value.var_99 * (1 + Math.sin(i / 5) * 0.3))
    dd.push(riskMetrics.value.max_drawdown * (1 + Math.sin(i / 6) * 0.2))
  }
  chart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['VaR(95%)', 'VaR(99%)', '最大回撤'], bottom: 0 },
    grid: { left: 60, right: 20, bottom: 40, top: 20 },
    xAxis: { type: 'category', data: dates },
    yAxis: { type: 'value', axisLabel: { formatter: (v: number) => (v * 100).toFixed(0) + '%' } },
    series: [
      { name: 'VaR(95%)', type: 'line', data: var95, smooth: true, lineStyle: { width: 2 }, itemStyle: { color: '#409EFF' } },
      { name: 'VaR(99%)', type: 'line', data: var99, smooth: true, lineStyle: { width: 2 }, itemStyle: { color: '#E6A23C' } },
      { name: '最大回撤', type: 'line', data: dd, smooth: true, lineStyle: { width: 2 }, itemStyle: { color: '#F56C6C' } },
    ],
  })
  window.addEventListener('resize', () => chart.resize())
}

// Initialize on mount
onMounted(async () => {
  fetchRiskMetrics();
  fetchRiskConfig();
  fetchRiskAlerts();
  await nextTick()
  initRiskChart();
});
</script>

<style scoped>
.risk-management {
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

.risk-metrics-card {
  margin-bottom: 20px;
}

.risk-stat-card {
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
  font-size: 18px;
  font-weight: bold;
  color: #333;
}

.risk-config-card {
  margin-bottom: 20px;
}

.pre-trade-check-card {
  margin-bottom: 20px;
}

.risk-alerts-card {
  margin-bottom: 20px;
}
</style>