<template>
  <el-card class="settings-card">
    <template #header>
      <div class="card-header">
        <span>OKX 交易所状态</span>
        <el-button size="small" @click="fetchConnStatus" :loading="checking">检测连接</el-button>
      </div>
    </template>

    <div v-if="connStatus" class="okx-status-grid">
      <div class="okx-status-field">
        <span class="field-label">连接状态</span>
        <el-tag :type="connStatus.connected ? 'success' : 'danger'" size="large">
          {{ connStatus.connected ? '已连接' : '未连接' }}
        </el-tag>
      </div>
      <div class="okx-status-field">
        <span class="field-label">模拟交易</span>
        <span>{{ connStatus.demo_trading ? '是' : '否' }}</span>
      </div>
      <div class="okx-status-field">
        <span class="field-label">交易所时间</span>
        <span>{{ connStatus.exchange_time || '-' }}</span>
      </div>
      <div class="okx-status-field">
        <span class="field-label">消息</span>
        <span>{{ connStatus.message || '-' }}</span>
      </div>
    </div>
    <div v-else class="okx-status-placeholder">
      <p>点击「检测连接」查看 OKX 交易所状态</p>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { checkOkxStatus } from '@/services/api'

defineOptions({ name: 'SettingsExchange' })

interface OkxStatusResponse {
  connected: boolean
  demo_trading: boolean
  exchange_time: string | null
  message: string | null
  [key: string]: unknown
}

const connStatus = ref<OkxStatusResponse | null>(null)
const checking = ref(false)

async function fetchConnStatus() {
  checking.value = true
  try {
    const data = await checkOkxStatus()
    connStatus.value = data as unknown as OkxStatusResponse
  } catch (error) {
    console.error('Failed to check OKX status:', error)
    ElMessage.error('检测 OKX 连接失败')
  } finally {
    checking.value = false
  }
}
</script>

<style scoped>
.settings-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.okx-status-grid {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 12px 0;
}

.okx-status-field {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 8px;
}

.okx-status-field .field-label {
  color: #909399;
  font-size: 14px;
}

.okx-status-placeholder {
  text-align: center;
  padding: 40px 20px;
  color: #909399;
}
</style>
