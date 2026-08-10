<template>
  <div class="risk-management">
    <el-row :gutter="20" class="header">
      <el-col :span="24">
        <h2>风险管理</h2>
      </el-col>
    </el-row>

    <RiskMetricsCards :metrics="riskMetrics" @refresh="fetchRiskMetrics" />
    <RiskChart />
    <RiskConfigForm :config="riskConfig" :saving="saving" @save="saveConfig" />
    <PreTradeCheckForm />
    <RiskAlertsTable :alerts="riskAlerts" @acknowledge="acknowledgeAlert" @refresh="fetchRiskAlerts" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import type { Alert, RiskConfig } from '@/services/types'
import { getRiskMetrics, getRiskConfig, updateRiskConfig } from '@/services/risk'
import { getAlerts, acknowledgeAlert as apiAcknowledgeAlert } from '@/services/monitor'

import RiskMetricsCards from '@/components/risk/RiskMetricsCards.vue'
import RiskChart from '@/components/risk/RiskChart.vue'
import RiskConfigForm from '@/components/risk/RiskConfigForm.vue'
import PreTradeCheckForm from '@/components/risk/PreTradeCheckForm.vue'
import RiskAlertsTable from '@/components/risk/RiskAlertsTable.vue'

const riskMetrics = ref<Record<string, number>>({
  var_95: 0.02,
  var_99: 0.04,
  max_position_size: 0.2,
  max_daily_loss: 0.05,
  max_drawdown: 0.15,
})

const riskConfig = ref<RiskConfig>({
  max_position_size: 0.2,
  max_daily_loss: 0.05,
  max_drawdown: 0.15,
  max_concentration: 0.2,
  enable_pre_trade_check: true,
  enable_real_time_monitor: true,
  var_confidence_level: 0.95,
})

const riskAlerts = ref<Alert[]>([])
const saving = ref(false)

async function fetchRiskMetrics() {
  try {
    riskMetrics.value = await getRiskMetrics()
  } catch (error) {
    console.error('Failed to fetch risk metrics:', error)
    ElMessage.error('获取风险指标失败')
  }
}

async function fetchRiskConfig() {
  try {
    riskConfig.value = await getRiskConfig()
  } catch (error) {
    console.error('Failed to fetch risk config:', error)
    ElMessage.error('获取风险配置失败')
  }
}

async function fetchRiskAlerts() {
  try {
    riskAlerts.value = await getAlerts()
  } catch (error) {
    console.error('Failed to fetch risk alerts:', error)
    ElMessage.error('获取风险告警失败')
  }
}

async function saveConfig(config: RiskConfig) {
  saving.value = true
  try {
    await updateRiskConfig(config)
    riskConfig.value = config
    ElMessage.success('风险配置保存成功')
  } catch (error) {
    console.error('Failed to save risk config:', error)
    ElMessage.error('保存风险配置失败: ' + (error as Error).message)
  } finally {
    saving.value = false
  }
}

async function acknowledgeAlert(alertId: number) {
  try {
    await apiAcknowledgeAlert(alertId)
    ElMessage.success('告警确认成功')
    await fetchRiskAlerts()
  } catch (error) {
    console.error('Failed to acknowledge alert:', error)
    ElMessage.error('告警确认失败: ' + (error as Error).message)
  }
}

onMounted(async () => {
  fetchRiskMetrics()
  fetchRiskConfig()
  fetchRiskAlerts()
})

defineExpose({
  riskMetrics,
  riskConfig,
  riskAlerts,
  saving,
  fetchRiskMetrics,
  fetchRiskConfig,
  fetchRiskAlerts,
  saveConfig,
  acknowledgeAlert,
})
</script>

<style scoped>
.risk-management {
  padding: 20px;
}
.header {
  margin-bottom: 20px;
}
</style>
