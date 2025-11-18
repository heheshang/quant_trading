<template>
  <div class="trading-system">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>交易执行</h2>
      </el-col>
    </el-row>

    <!-- 账户信息 -->
    <el-card class="account-info-card">
      <template #header>
        <div class="card-header">
          <span>账户信息</span>
        </div>
      </template>
      
      <el-row :gutter="20">
        <el-col :span="6">
          <div class="account-stat">
            <div class="stat-label">总资产</div>
            <div class="stat-value">¥{{ formatCurrency(accountInfo.total_assets) }}</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="account-stat">
            <div class="stat-label">可用资金</div>
            <div class="stat-value">¥{{ formatCurrency(accountInfo.available_cash) }}</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="account-stat">
            <div class="stat-label">持仓市值</div>
            <div class="stat-value">¥{{ formatCurrency(accountInfo.market_value) }}</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="account-stat">
            <div class="stat-label">当日盈亏</div>
            <div class="stat-value" :class="{ positive: accountInfo.daily_pnl > 0, negative: accountInfo.daily_pnl < 0 }">
              ¥{{ formatCurrency(accountInfo.daily_pnl) }}
            </div>
          </div>
        </el-col>
      </el-row>
    </el-card>

    <!-- 订单创建 -->
    <el-card class="order-form-card">
      <template #header>
        <div class="card-header">
          <span>创建订单</span>
        </div>
      </template>
      
      <el-form :model="orderForm" label-width="100px" :rules="orderRules" ref="orderFormRef">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="策略" prop="strategy_id">
              <el-select v-model="orderForm.strategy_id" placeholder="选择策略" style="width: 100%">
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
            <el-form-item label="标的代码" prop="symbol">
              <el-input v-model="orderForm.symbol" placeholder="输入标的代码，如 600519.SH" />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="买卖方向" prop="side">
              <el-select v-model="orderForm.side" placeholder="选择买卖方向" style="width: 100%">
                <el-option label="买入" value="Buy" />
                <el-option label="卖出" value="Sell" />
              </el-select>
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="订单类型" prop="order_type">
              <el-select v-model="orderForm.order_type" placeholder="选择订单类型" style="width: 100%">
                <el-option label="限价单" value="Limit" />
                <el-option label="市价单" value="Market" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="价格" prop="price">
              <el-input-number 
                v-model="orderForm.price" 
                :min="0" 
                :precision="2" 
                :step="0.01" 
                style="width: 100%" 
                :disabled="orderForm.order_type === 'Market'"
              />
            </el-form-item>
          </el-col>
          
          <el-col :span="12">
            <el-form-item label="数量" prop="quantity">
              <el-input-number 
                v-model="orderForm.quantity" 
                :min="0" 
                :precision="2" 
                :step="100" 
                style="width: 100%" 
              />
            </el-form-item>
          </el-col>
        </el-row>
        
        <el-form-item>
          <el-button type="primary" @click="submitOrder" :loading="submitting">提交订单</el-button>
          <el-button @click="resetOrderForm">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 持仓信息 -->
    <el-card class="positions-card">
      <template #header>
        <div class="card-header">
          <span>持仓信息</span>
        </div>
      </template>
      
      <el-table :data="positions" style="width: 100%">
        <el-table-column prop="symbol" label="标的代码" width="120" />
        <el-table-column prop="quantity" label="持仓数量" width="120" />
        <el-table-column prop="available_quantity" label="可用数量" width="120" />
        <el-table-column prop="avg_price" label="成本价" width="120">
          <template #default="scope">
            ¥{{ scope.row.avg_price.toFixed(2) }}
          </template>
        </el-table-column>
        <el-table-column prop="market_value" label="市值" width="120">
          <template #default="scope">
            ¥{{ formatCurrency(scope.row.market_value) }}
          </template>
        </el-table-column>
        <el-table-column prop="unrealized_pnl" label="浮动盈亏" width="120">
          <template #default="scope">
            <span :class="{ positive: scope.row.unrealized_pnl > 0, negative: scope.row.unrealized_pnl < 0 }">
              ¥{{ formatCurrency(scope.row.unrealized_pnl) }}
            </span>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 活跃订单 -->
    <el-card class="active-orders-card">
      <template #header>
        <div class="card-header">
          <span>活跃订单</span>
          <el-button @click="refreshOrders">刷新</el-button>
        </div>
      </template>
      
      <el-table :data="activeOrders" style="width: 100%">
        <el-table-column prop="order_id" label="订单ID" width="200" />
        <el-table-column prop="strategy_id" label="策略" width="120" />
        <el-table-column prop="symbol" label="标的" width="120" />
        <el-table-column prop="side" label="方向" width="80">
          <template #default="scope">
            <el-tag :type="scope.row.side === 'Buy' ? 'success' : 'danger'">
              {{ scope.row.side === 'Buy' ? '买入' : '卖出' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="order_type" label="类型" width="80">
          <template #default="scope">
            {{ scope.row.order_type === 'Limit' ? '限价' : '市价' }}
          </template>
        </el-table-column>
        <el-table-column prop="price" label="价格" width="100">
          <template #default="scope">
            <span v-if="scope.row.price">¥{{ scope.row.price.toFixed(2) }}</span>
            <span v-else>市价</span>
          </template>
        </el-table-column>
        <el-table-column prop="quantity" label="数量" width="100" />
        <el-table-column prop="filled_quantity" label="已成交" width="100" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="scope">
            <el-tag :type="getOrderStatusType(scope.row.status)">
              {{ getOrderStatusText(scope.row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120">
          <template #default="scope">
            <el-button 
              size="small" 
              type="danger" 
              @click="cancelOrder(scope.row.order_id)"
              :disabled="scope.row.status !== 'Submitted' && scope.row.status !== 'PartiallyFilled'"
            >
              撤单
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
import { ElMessage, ElMessageBox, FormInstance } from 'element-plus';

// Reactive data
const accountInfo = ref({
  account_id: '',
  total_assets: 1234567.91,
  available_cash: 234567.99,
  frozen_cash: 0,
  market_value: 1000000,
  total_pnl: 12345.67,
  daily_pnl: 12345.67,
  margin: 0,
  margin_ratio: 0,
  updated_at: new Date()
});

const positions = ref<any[]>([]);
const activeOrders = ref<any[]>([]);
const strategies = ref<any[]>([]);

const orderForm = ref({
  strategy_id: '',
  symbol: '600519.SH',
  side: 'Buy',
  order_type: 'Limit',
  price: 1685.00,
  quantity: 100
});

const orderRules = {
  strategy_id: [{ required: true, message: '请选择策略', trigger: 'change' }],
  symbol: [{ required: true, message: '请输入标的代码', trigger: 'blur' }],
  side: [{ required: true, message: '请选择买卖方向', trigger: 'change' }],
  order_type: [{ required: true, message: '请选择订单类型', trigger: 'change' }],
  price: [{ required: true, message: '请输入价格', trigger: 'blur' }],
  quantity: [{ required: true, message: '请输入数量', trigger: 'blur' }]
};

const orderFormRef = ref<FormInstance>();
const submitting = ref(false);

// Format currency
function formatCurrency(value: any): string {
  if (!value) return '0.00';
  return parseFloat(value.toString()).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

// Get order status type for tag
function getOrderStatusType(status: string): string {
  switch (status) {
    case 'Pending':
      return '';
    case 'Submitted':
      return 'primary';
    case 'PartiallyFilled':
      return 'warning';
    case 'Filled':
      return 'success';
    case 'Cancelled':
      return 'danger';
    default:
      return 'info';
  }
}

// Get order status text
function getOrderStatusText(status: string): string {
  switch (status) {
    case 'Pending':
      return '待提交';
    case 'Submitted':
      return '已提交';
    case 'PartiallyFilled':
      return '部分成交';
    case 'Filled':
      return '已成交';
    case 'Cancelled':
      return '已撤单';
    default:
      return status;
  }
}

// Generate a simple UUID-like string
function generateOrderId(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
    const r = Math.random() * 16 | 0;
    const v = c == 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

// Fetch account info
async function fetchAccountInfo() {
  try {
    const data = await invoke<any>('get_account_info');
    accountInfo.value = data;
  } catch (error) {
    console.error('Failed to fetch account info:', error);
    ElMessage.error('获取账户信息失败');
  }
}

// Fetch positions
async function fetchPositions() {
  try {
    const data = await invoke<any[]>('get_positions');
    positions.value = data;
  } catch (error) {
    console.error('Failed to fetch positions:', error);
    ElMessage.error('获取持仓信息失败');
  }
}

// Fetch active orders
async function fetchActiveOrders() {
  try {
    const data = await invoke<any[]>('get_active_orders');
    activeOrders.value = data;
  } catch (error) {
    console.error('Failed to fetch active orders:', error);
    ElMessage.error('获取活跃订单失败');
  }
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

// Submit order
async function submitOrder() {
  if (!orderFormRef.value) return;
  
  await orderFormRef.value.validate(async (valid) => {
    if (!valid) return;
    
    submitting.value = true;
    try {
      const order = {
        order_id: generateOrderId(),
        strategy_id: orderForm.value.strategy_id,
        symbol: orderForm.value.symbol,
        order_type: orderForm.value.order_type,
        side: orderForm.value.side,
        price: orderForm.value.order_type === 'Limit' ? orderForm.value.price : null,
        quantity: orderForm.value.quantity,
        filled_quantity: 0,
        status: 'Pending',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        commission: 0,
        slippage: 0
      };
      
      const orderId = await invoke<string>('submit_order', { order });
      ElMessage.success(`订单提交成功: ${orderId}`);
      resetOrderForm();
      await fetchActiveOrders();
    } catch (error) {
      console.error('Failed to submit order:', error);
      ElMessage.error('订单提交失败: ' + (error as Error).message);
    } finally {
      submitting.value = false;
    }
  });
}

// Cancel order
async function cancelOrder(orderId: string) {
  try {
    await ElMessageBox.confirm('确定要撤销此订单吗？', '确认撤单', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    });
    
    // TODO: Implement cancel order functionality
    ElMessage.info('撤单功能开发中...');
    await fetchActiveOrders();
  } catch {
    // User cancelled
  }
}

// Reset order form
function resetOrderForm() {
  orderForm.value = {
    strategy_id: '',
    symbol: '600519.SH',
    side: 'Buy',
    order_type: 'Limit',
    price: 1685.00,
    quantity: 100
  };
}

// Refresh orders
async function refreshOrders() {
  await fetchActiveOrders();
  ElMessage.success('刷新成功');
}

// Initialize on mount
onMounted(() => {
  fetchAccountInfo();
  fetchPositions();
  fetchActiveOrders();
  fetchStrategies();
});
</script>

<style scoped>
.trading-system {
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

.account-info-card {
  margin-bottom: 20px;
}

.account-stat {
  text-align: center;
  padding: 10px 0;
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

.stat-value.positive {
  color: #67C23A;
}

.stat-value.negative {
  color: #F56C6C;
}

.order-form-card {
  margin-bottom: 20px;
}

.positions-card {
  margin-bottom: 20px;
}

.active-orders-card {
  margin-bottom: 20px;
}
</style>