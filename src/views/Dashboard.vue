<template>
  <div class="dashboard">
    <el-alert
      v-if="error"
      :title="error"
      type="error"
      show-icon
      closable
      @close="error = null"
      style="margin-bottom: 20px"
    />

    <el-row :gutter="20">
      <el-col :span="24">
        <div
          style="
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 20px;
          "
        >
          <h2 style="margin: 0">仪表盘</h2>
          <el-button type="primary" @click="refreshData" :loading="loading"
            >刷新数据</el-button
          >
        </div>
      </el-col>
    </el-row>

    <el-skeleton
      v-if="loading && !accountInfo.total_assets"
      :rows="5"
      animated
    />

    <div v-else>
      <el-row :gutter="20">
        <el-col :span="6">
          <el-card class="stat-card">
            <div class="stat-item">
              <div class="stat-icon" style="background: #409eff">
                <el-icon><TrendCharts /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-label">总资产</div>
                <div class="stat-value">
                  ¥{{ formatNumber(accountInfo.total_assets) }}
                </div>
              </div>
            </div>
          </el-card>
        </el-col>

        <el-col :span="6">
          <el-card class="stat-card">
            <div class="stat-item">
              <div class="stat-icon" style="background: #67c23a">
                <el-icon><Promotion /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-label">今日收益</div>
                <div
                  class="stat-value"
                  :class="{
                    positive: accountInfo.daily_pnl > 0,
                    negative: accountInfo.daily_pnl < 0,
                  }"
                >
                  {{ accountInfo.daily_pnl > 0 ? "+" : "" }}¥{{
                    formatNumber(accountInfo.daily_pnl)
                  }}
                </div>
              </div>
            </div>
          </el-card>
        </el-col>

        <el-col :span="6">
          <el-card class="stat-card">
            <div class="stat-item">
              <div class="stat-icon" style="background: #e6a23c">
                <el-icon><Tickets /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-label">活跃订单</div>
                <div class="stat-value">{{ activeOrders.length }}</div>
              </div>
            </div>
          </el-card>
        </el-col>

        <el-col :span="6">
          <el-card class="stat-card">
            <div class="stat-item">
              <div class="stat-icon" style="background: #f56c6c">
                <el-icon><Warning /></el-icon>
              </div>
              <div class="stat-info">
                <div class="stat-label">风险等级</div>
                <div class="stat-value">中</div>
              </div>
            </div>
          </el-card>
        </el-col>
      </el-row>

      <el-row :gutter="20" style="margin-top: 20px">
        <el-col :span="16">
          <el-card>
            <template #header>
              <div class="card-header">
                <span>资产曲线</span>
              </div>
            </template>
            <div id="equity-chart" style="height: 400px"></div>
          </el-card>
        </el-col>

        <el-col :span="8">
          <el-card>
            <template #header>
              <div class="card-header">
                <span>持仓分布</span>
              </div>
            </template>
            <div id="position-chart" style="height: 400px"></div>
          </el-card>
        </el-col>
      </el-row>

      <el-row :gutter="20" style="margin-top: 20px">
        <el-col :span="24">
          <el-card>
            <template #header>
              <div class="card-header">
                <span>最近交易</span>
              </div>
            </template>
            <el-table :data="recentTrades" style="width: 100%">
              <el-table-column prop="time" label="时间" width="180" />
              <el-table-column prop="symbol" label="标的" width="120" />
              <el-table-column prop="side" label="方向" width="100">
                <template #default="scope">
                  <el-tag
                    :type="scope.row.side === '买入' ? 'success' : 'danger'"
                  >
                    {{ scope.row.side }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="price" label="价格" />
              <el-table-column prop="quantity" label="数量" />
              <el-table-column prop="status" label="状态">
                <template #default="scope">
                  <el-tag
                    :type="scope.row.status === '已成交' ? 'success' : 'info'"
                  >
                    {{ scope.row.status }}
                  </el-tag>
                </template>
              </el-table-column>
            </el-table>
          </el-card>
        </el-col>
      </el-row>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import * as echarts from "echarts";
import {
  TrendCharts,
  Promotion,
  Tickets,
  Warning,
} from "@element-plus/icons-vue";
import { invoke } from "@tauri-apps/api/core";
// Define reactive data
const accountInfo = ref({
  total_assets: 0,
  daily_pnl: 0,
  available_cash: 0,
  market_value: 0,
  total_pnl: 0,
});

const positions = ref<any[]>([]);
const activeOrders = ref<any[]>([]);
const recentTrades = ref<any[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

// Format numbers for display
function formatNumber(value: any) {
  if (!value) return "0.00";
  return parseFloat(value.toString()).toLocaleString("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

// Fetch account information
async function fetchAccountInfo() {
  try {
    // Check if we're in a Tauri environment
    // @ts-ignore
    const data: any = await invoke("get_account_info");
    accountInfo.value = data;
    console.log("Account info:", data);
    // if (window.__TAURI__ && window.__TAURI__.invoke) {
    //   // @ts-ignore
    //   const data: any = await window.__TAURI__.invoke('get_account_info');
    //   accountInfo.value = data;
    //   console.log('Account info:', data);
    // } else {
    //   // Fallback mock data for web development
    //   accountInfo.value = {
    //     total_assets: 1234567.89,
    //     daily_pnl: 12345.67,
    //     available_cash: 234567.89,
    //     market_value: 1000000,
    //     total_pnl: 12345.67
    //   };
    // console.log("Using mock account data for development");
    // }
  } catch (err: any) {
    console.error("Failed to fetch account info:", err);
    error.value = "获取账户信息失败";
  }
}

// Fetch positions
async function fetchPositions() {
  try {
    const data: any = await invoke("get_positions");
    positions.value = data;
    console.log("Positions:", data);
  } catch (err: any) {
    console.error("Failed to fetch positions:", err);
    error.value = "获取持仓信息失败";
  }
}

// Fetch active orders
async function fetchActiveOrders() {
  try {
    const data: any = await invoke("get_active_orders");
    activeOrders.value = data;
    console.log("Active orders:", data);

    // Convert to display format
    recentTrades.value = data.map((order: any) => ({
      time: new Date(order.created_at).toLocaleString("zh-CN"),
      symbol: order.symbol,
      side:
        order.side === "Buy"
          ? "买入"
          : order.side === "Sell"
          ? "卖出"
          : order.side,
      price: order.price ? formatNumber(order.price) : "-",
      quantity: order.quantity.toString(),
      status: getOrderStatusDisplay(order.status),
    }));
    // Check if we're in a Tauri environment
    // @ts-ignore
    // if (window.__TAURI__ && window.__TAURI__.invoke) {
    //   // @ts-ignore
    //   const data: any = await window.__TAURI__.invoke("get_active_orders");
    //   activeOrders.value = data;
    //   console.log("Active orders:", data);

    //   // Convert to display format
    //   recentTrades.value = data.map((order: any) => ({
    //     time: new Date(order.created_at).toLocaleString("zh-CN"),
    //     symbol: order.symbol,
    //     side:
    //       order.side === "Buy"
    //         ? "买入"
    //         : order.side === "Sell"
    //         ? "卖出"
    //         : order.side,
    //     price: order.price ? formatNumber(order.price) : "-",
    //     quantity: order.quantity.toString(),
    //     status: getOrderStatusDisplay(order.status),
    //   }));
    // } else {
    //   // Fallback mock data for web development
    //   const mockData = [
    //     {
    //       order_id: "12345",
    //       strategy_id: "trend_following",
    //       symbol: "600519.SH",
    //       order_type: "Limit",
    //       side: "Buy",
    //       price: 1685.0,
    //       quantity: 100,
    //       filled_quantity: 0,
    //       status: "Submitted",
    //       created_at: new Date().toISOString(),
    //       updated_at: new Date().toISOString(),
    //       commission: 0,
    //       slippage: 0,
    //     },
    //   ];

    //   activeOrders.value = mockData;
    //   console.log("Using mock orders data for development");

    //   // Convert to display format
    //   recentTrades.value = mockData.map((order: any) => ({
    //     time: new Date(order.created_at).toLocaleString("zh-CN"),
    //     symbol: order.symbol,
    //     side:
    //       order.side === "Buy"
    //         ? "买入"
    //         : order.side === "Sell"
    //         ? "卖出"
    //         : order.side,
    //     price: order.price ? formatNumber(order.price) : "-",
    //     quantity: order.quantity.toString(),
    //     status: getOrderStatusDisplay(order.status),
    //   }));
    // }
  } catch (err: any) {
    console.error("Failed to fetch active orders:", err);
    error.value = "获取订单信息失败";
  }
}

// Convert order status to display text
function getOrderStatusDisplay(status: string) {
  switch (status) {
    case "Submitted":
      return "已提交";
    case "Filled":
      return "已成交";
    case "PartiallyFilled":
      return "部分成交";
    case "Cancelled":
      return "已撤单";
    case "Rejected":
      return "已拒绝";
    default:
      return status;
  }
}

// Initialize charts
function initCharts() {
  // Initialize equity chart with real data
  const equityChart = echarts.init(document.getElementById("equity-chart")!);

  // Create mock equity data based on account history
  // In a real implementation, this would come from backend
  const dates = [];
  const values = [];
  const today = new Date();

  // Generate 30 days of mock data
  for (let i = 29; i >= 0; i--) {
    const date = new Date(today);
    date.setDate(date.getDate() - i);
    dates.push(
      date.toLocaleDateString("zh-CN", { month: "short", day: "numeric" })
    );

    // Generate realistic equity curve
    const baseValue = 1200000;
    const fluctuation = Math.sin(i / 5) * 50000 + Math.random() * 20000;
    values.push(baseValue + fluctuation);
  }

  equityChart.setOption({
    tooltip: {
      trigger: "axis",
      formatter: function (params: any) {
        return `${params[0].axisValue}<br/>¥${formatNumber(params[0].value)}`;
      },
    },
    xAxis: {
      type: "category",
      data: dates,
    },
    yAxis: {
      type: "value",
      axisLabel: {
        formatter: function (value: number) {
          return "¥" + (value / 10000).toFixed(0) + "万";
        },
      },
    },
    series: [
      {
        data: values,
        type: "line",
        smooth: true,
        areaStyle: {},
        lineStyle: { width: 3 },
        itemStyle: { color: "#409EFF" },
      },
    ],
  });

  // Initialize position chart
  const positionChart = echarts.init(
    document.getElementById("position-chart")!
  );

  if (positions.value.length > 0) {
    const chartData = positions.value.map((pos) => ({
      value: parseFloat(pos.market_value.toString()),
      name: pos.symbol,
    }));

    positionChart.setOption({
      tooltip: {
        trigger: "item",
        formatter: function (params: any) {
          return `${params.name}<br/>¥${formatNumber(params.value)} (${
            params.percent
          }%)`;
        },
      },
      series: [
        {
          type: "pie",
          radius: ["40%", "70%"],
          data: chartData,
          emphasis: {
            itemStyle: {
              shadowBlur: 10,
              shadowOffsetX: 0,
              shadowColor: "rgba(0, 0, 0, 0.5)",
            },
          },
        },
      ],
    });
  } else {
    // Show message when no positions
    positionChart.setOption({
      graphic: {
        elements: [
          {
            type: "text",
            key: "no-data",
            style: {
              text: "暂无持仓",
              fontSize: 16,
              textAlign: "center",
              fill: "#999",
            },
            position: ["50%", "50%"],
            shape: {
              r: 10,
            },
          },
        ],
      },
    });
  }
}

// Refresh all data
async function refreshData() {
  try {
    loading.value = true;
    error.value = null;
    await Promise.all([
      fetchAccountInfo(),
      fetchPositions(),
      fetchActiveOrders(),
    ]);
    initCharts();
  } catch (err: any) {
    console.error("Error refreshing data:", err);
    error.value = "刷新数据失败";
  } finally {
    loading.value = false;
  }
}

// Watch for data changes and reinitialize charts
watch([accountInfo, positions, activeOrders], () => {
  setTimeout(() => {
    initCharts();
  }, 100);
});

// Initialize on mount
onMounted(async () => {
  await refreshData();

  // Refresh data every 30 seconds
  setInterval(() => {
    refreshData();
  }, 30000);
});
</script>

<style scoped>
.dashboard {
  padding: 0;
}

.stat-card {
  margin-bottom: 20px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 20px;
}

.stat-icon {
  width: 60px;
  height: 60px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: #fff;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #999;
  margin-bottom: 8px;
}

.stat-value {
  font-size: 24px;
  font-weight: bold;
  color: #333;
}

.stat-value.positive {
  color: #67c23a;
}

.stat-value.negative {
  color: #f56c6c;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
