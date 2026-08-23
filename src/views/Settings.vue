<template>
  <div class="system-settings">
    <el-row :gutter="20" class="header"><el-col :span="24"><h2>系统设置</h2></el-col></el-row>
    <el-tabs v-model="activeTab" class="settings-tabs">
      <el-tab-pane label="基本设置" name="basic"><SettingsBasic ref="basicRef" v-model="systemInfo" /></el-tab-pane>
      <el-tab-pane label="数据库" name="database"><SettingsDatabase ref="dbRef" v-model="config.database" /></el-tab-pane>
      <el-tab-pane label="缓存" name="redis"><SettingsRedis ref="redisRef" v-model="config.redis" /></el-tab-pane>
      <el-tab-pane label="交易所" name="exchange"><SettingsExchange /></el-tab-pane>
      <el-tab-pane label="交易" name="trading"><SettingsTrading ref="tradingRef" v-model="config.trading" /></el-tab-pane>
      <el-tab-pane label="风险" name="risk"><SettingsRisk ref="riskRef" v-model="config.risk" /></el-tab-pane>
      <el-tab-pane label="监控" name="monitoring"><SettingsMonitoring ref="monitoringRef" v-model="config.monitoring" /></el-tab-pane>
      <el-tab-pane label="安全" name="security"><SettingsSecurity ref="securityRef" v-model="config.security" /></el-tab-pane>
    </el-tabs>
    <div class="action-bar">
      <el-button type="primary" @click="saveConfig" :loading="saving">保存配置</el-button>
      <el-button @click="resetConfig">重置</el-button>
      <el-button @click="exportConfig">导出配置</el-button>
      <el-button @click="triggerImport">导入配置</el-button>
      <input ref="importFileInput" type="file" accept=".json" style="display:none" @change="handleImport" />
    </div>
    <ConfirmDialog v-model:visible="resetDialogVisible" title="确认重置"
      message="确定要重置所有系统设置吗？此操作不可撤销。" type="danger" confirm-text="重置" @confirm="confirmReset" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getConfig, updateConfig } from '@/services/config'
import type { AppConfig } from '@/services/types'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import ConfirmDialog from '@/components/common/ConfirmDialog.vue'
import SettingsBasic from '@/components/settings/SettingsBasic.vue'
import SettingsDatabase from '@/components/settings/SettingsDatabase.vue'
import SettingsRedis from '@/components/settings/SettingsRedis.vue'
import SettingsExchange from '@/components/settings/SettingsExchange.vue'
import SettingsTrading from '@/components/settings/SettingsTrading.vue'
import SettingsRisk from '@/components/settings/SettingsRisk.vue'
import SettingsMonitoring from '@/components/settings/SettingsMonitoring.vue'
import SettingsSecurity from '@/components/settings/SettingsSecurity.vue'

const basicRef = ref<InstanceType<typeof SettingsBasic>>()
const dbRef = ref<InstanceType<typeof SettingsDatabase>>()
const redisRef = ref<InstanceType<typeof SettingsRedis>>()
const tradingRef = ref<InstanceType<typeof SettingsTrading>>()
const riskRef = ref<InstanceType<typeof SettingsRisk>>()
const monitoringRef = ref<InstanceType<typeof SettingsMonitoring>>()
const securityRef = ref<InstanceType<typeof SettingsSecurity>>()

const activeTab = ref('basic')
const saving = ref(false)
const resetDialogVisible = ref(false)

interface SystemInfo { name: string; version: string; language: string; timezone: string }
const systemInfo = ref<SystemInfo>({ name: '量化交易系统', version: '1.0.0', language: 'zh-CN', timezone: 'UTC+8' })

const config = ref({
  database: { host: 'localhost', port: 5432, username: '', password: null as string | null, database: 'quant_trading', max_connections: 50, connect_timeout_seconds: 3 },
  redis: { host: 'localhost', port: 6379, password: null as string | null, db: 0, pool_size: 20 },
  trading: { enable_paper_trading: false, max_orders_per_second: 100, default_commission_rate: 0.0003 as number, default_slippage: 0.0001 as number, order_timeout_seconds: 30 },
  risk: { max_position_size: 0.2, max_daily_loss: 0.05, max_drawdown: 0.15, max_concentration: 0.2, enable_pre_trade_check: true, enable_real_time_monitor: true, var_confidence_level: 0.95 },
  monitoring: { enable_prometheus: true, prometheus_port: 9090, log_level: 'info', alert_email: null as string | null, alert_webhook: null as string | null },
  security: { enable_encryption: true, jwt_secret: '', token_expiry_hours: 24, enable_2fa: false, allowed_ips: ['127.0.0.1'] },
})

function mergeConfig<T extends Record<string, unknown>>(target: T, source: Partial<T>): T {
  const result = { ...target }
  for (const key of Object.keys(source) as Array<keyof T>) {
    const sv = source[key]
    if (sv !== null && typeof sv === 'object' && !Array.isArray(sv)) {
      result[key] = { ...(result[key] as Record<string, unknown>), ...(sv as Record<string, unknown>) } as T[keyof T]
    } else if (sv !== undefined) {
      result[key] = sv as T[keyof T]
    }
  }
  return result
}

async function fetchConfig() {
  try {
    config.value = mergeConfig(config.value, await getConfig() as Partial<typeof config.value>)
  } catch (error) {
    console.error('Failed to fetch config:', error)
    ElMessage.error('获取配置失败')
  }
}

function buildAppPayload(cfg: typeof config.value, sys: SystemInfo): AppConfig {
  return {
    app_name: sys.name, version: sys.version, debug: false,
    database: { ...cfg.database, password: cfg.database.password ?? '' },
    redis: { ...cfg.redis, password: cfg.redis.password ?? '' },
    trading: cfg.trading, risk: cfg.risk,
    monitoring: { ...cfg.monitoring, alert_email: cfg.monitoring.alert_email ?? '', alert_webhook: cfg.monitoring.alert_webhook ?? '' },
    security: cfg.security,
  }
}

async function saveConfig() {
  const formRefs = [basicRef, dbRef, redisRef, tradingRef, riskRef, monitoringRef, securityRef]
    .map(r => r.value?.formRef)
    .filter((r): r is FormInstance => r !== undefined)
  if (formRefs.length === 0) return
  const allValid = (await Promise.all(formRefs.map(r => r.validate().then(() => true).catch(() => false)))).every(Boolean)
  if (!allValid) { ElMessage.warning('请检查表单中的必填项'); return }
  saving.value = true
  try {
    await updateConfig(buildAppPayload(config.value, systemInfo.value))
    ElMessage.success('配置保存成功')
  } catch (error) {
    console.error('Failed to save config:', error)
    ElMessage.error('保存配置失败: ' + (error as Error).message)
  } finally { saving.value = false }
}

function resetConfig() { resetDialogVisible.value = true }

async function confirmReset() {
  try { await fetchConfig(); ElMessage.success('设置已重置'); resetDialogVisible.value = false }
  catch (error) { console.error('Failed to reset config:', error); ElMessage.error('重置失败: ' + (error as Error).message) }
}

function exportConfig() {
  const payload = buildAppPayload(config.value, systemInfo.value)
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url; a.download = `quant-trader-config-${new Date().toISOString().slice(0, 10)}.json`; a.click()
  URL.revokeObjectURL(url)
  ElMessage.success('配置已导出')
}

const importFileInput = ref<HTMLInputElement | null>(null)
function triggerImport() { importFileInput.value?.click() }

async function handleImport(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  try {
    const imported: Record<string, unknown> = JSON.parse(await file.text())
    const missing = ['database', 'trading'].filter(s => !imported[s] || typeof imported[s] !== 'object' || Array.isArray(imported[s]))
    if (missing.length) { ElMessage.error(`无效的配置文件：缺少必需配置节 [${missing.join(', ')}]`); return }
    const dbSection = imported.database as Record<string, unknown>
    if (!dbSection.host || !dbSection.port) { ElMessage.error('无效的配置文件：database 节缺少 host 或 port'); return }
    const trSection = imported.trading as Record<string, unknown>
    if (typeof trSection.max_position_size !== 'number' && typeof trSection.max_order_size !== 'number') {
      ElMessage.warning('配置文件缺少交易限额设置，将使用默认值')
    }
    const sysSection = imported.system as Record<string, unknown> | undefined
    if (sysSection?.env && !['dev', 'test', 'staging', 'prod'].includes(sysSection.env as string)) {
      ElMessage.warning(`未知环境 "${String(sysSection.env)}"，将使用当前配置`); sysSection.env = undefined
    }
    config.value = mergeConfig(config.value, {
      database: imported.database as typeof config.value.database,
      trading: imported.trading as typeof config.value.trading,
      risk: imported.risk as typeof config.value.risk,
    })
    ElMessage.success('配置已导入，请点击"保存配置"以生效')
  } catch (err) { ElMessage.error('导入失败: ' + (err as Error).message) }
  finally { input.value = '' }
}

onMounted(() => { fetchConfig() })
</script>

<style scoped>
.system-settings { padding: 20px; }
.header { margin-bottom: 20px; }
.settings-tabs { margin-bottom: var(--space-md); }
.action-bar {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-sm);
  border-top: 1px solid var(--color-border-light);
}
@media (max-width: 768px) {
  .action-bar {
    flex-wrap: wrap;
  }
  .action-bar .el-button {
    flex: 1 1 auto;
  }
}
</style>
