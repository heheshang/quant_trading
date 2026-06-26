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

    <!-- 风险配置 -->
    <el-card class="risk-config-card">
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
          <el-button @click="refreshAlerts">刷新</el-button>
        </div>
      </template>
      
      <el-table :data="riskAlerts" style="width: 100%">
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
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage, FormInstance } from 'element-plus';

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
    const data = await invoke<any>('get_risk_metrics');
    riskMetrics.value = data;
  } catch (error) {
    console.error('Failed to fetch risk metrics:', error);
    ElMessage.error('获取风险指标失败');
  }
}

// Fetch risk config
async function fetchRiskConfig() {
  try {
    const data = await invoke<any>('get_risk_config');
    riskConfig.value = data;
  } catch (error) {
    console.error('Failed to fetch risk config:', error);
    ElMessage.error('获取风险配置失败');
  }
}

// Fetch risk alerts
async function fetchRiskAlerts() {
  try {
    const data = await invoke<any[]>('get_alerts');
    riskAlerts.value = data;
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
      await invoke<boolean>('update_risk_config', { config: riskConfig.value });
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
      
      const result = await invoke<boolean>('pre_trade_check', {
        order: testOrder.value,
        account,
        positions
      });
      
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
async function acknowledgeAlert(alertId: string) {
  try {
    await invoke<boolean>('acknowledge_alert', { alertId });
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

// Initialize on mount
onMounted(() => {
  fetchRiskMetrics();
  fetchRiskConfig();
  fetchRiskAlerts();
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