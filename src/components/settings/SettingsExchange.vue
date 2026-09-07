<template>
  <div>
    <el-card class="settings-card api-key-card">
      <template #header>
        <div class="card-header">
          <span>API 凭据</span>
        </div>
      </template>

      <el-form :model="credForm" :rules="credRules" ref="credFormRef" label-width="110px">
        <el-form-item label="API Key" prop="apiKey">
          <el-input v-model="credForm.apiKey" placeholder="输入 API Key" />
        </el-form-item>
        <el-form-item label="Secret" prop="secret">
          <el-input v-model="credForm.secret" type="password" show-password placeholder="输入 API Secret" />
        </el-form-item>
        <el-form-item label="环境" prop="environment">
          <el-radio-group v-model="credForm.environment">
            <el-radio label="demo">Demo (模拟)</el-radio>
            <el-radio label="live">Live (实盘)</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="saving" @click="handleSaveCredential">保存凭据</el-button>
        </el-form-item>
      </el-form>

      <el-divider />

      <div class="saved-keys-section">
        <div class="section-title">已保存的密钥</div>
        <el-empty v-if="keys.length === 0" description="暂无已保存的密钥" :image-size="80" />
        <el-table v-else :data="keys" size="small">
          <el-table-column prop="exchange" label="交易所" width="110" />
          <el-table-column prop="api_key" label="API Key" />
          <el-table-column prop="passphrase" label="Passphrase" />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
                {{ row.is_active ? '有效' : '停用' }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { saveApiKey, getApiKeys, type MaskedApiKey } from '@/services/apiKey'
import { useAuthStore } from '@/stores/auth'

defineOptions({ name: 'SettingsExchange' })

const ENVIRONMENT_STORAGE_KEY = 'api_key_environment'

const saving = ref(false)
const keys = ref<MaskedApiKey[]>([])
const authStore = useAuthStore()

const credFormRef = ref<FormInstance>()
const credForm = reactive({
  exchange: 'BINANCE',
  apiKey: '',
  secret: '',
  environment: 'demo',
})

const credRules = {
  apiKey: [{ required: true, message: '请输入 API Key', trigger: 'blur' }],
  secret: [{ required: true, message: '请输入 API Secret', trigger: 'blur' }],
}

function loadEnvironment() {
  try {
    const stored = localStorage.getItem(ENVIRONMENT_STORAGE_KEY)
    if (stored === 'demo' || stored === 'live') credForm.environment = stored
  } catch {
    // ignore
  }
}

function persistEnvironment() {
  try {
    localStorage.setItem(ENVIRONMENT_STORAGE_KEY, credForm.environment)
  } catch {
    // ignore
  }
}

async function fetchKeys() {
  const user = authStore.currentUser
  if (!user) {
    keys.value = []
    return
  }
  try {
    const data = await getApiKeys(user.id)
    if (authStore.currentUser !== user) return
    keys.value = Array.isArray(data) ? data : []
  } catch (error) {
    if (authStore.currentUser !== user) return
    console.error('Failed to fetch API keys:', error)
    keys.value = []
  }
}

async function handleSaveCredential() {
  if (!credFormRef.value) return
  const user = authStore.currentUser
  if (!user) {
    ElMessage.error('请先登录')
    return
  }
  try {
    await credFormRef.value.validate()
  } catch {
    return
  }
  if (authStore.currentUser !== user) return;
  saving.value = true
  try {
    const ok = await saveApiKey({
      user_id: user.id,
      exchange: 'BINANCE',
      api_key: credForm.apiKey,
      secret: credForm.secret,
      passphrase: null,
    })
    if (authStore.currentUser !== user) return
    if (ok) {
      ElMessage.success('API 凭据保存成功')
      credForm.secret = ''
      persistEnvironment()
      await fetchKeys()
    } else {
      ElMessage.error('API 凭据保存失败')
    }
  } catch (error) {
    if (authStore.currentUser !== user) return
    console.error('Failed to save API key:', error)
    ElMessage.error('保存 API 凭据失败: ' + (error as Error).message)
  } finally {
    if (authStore.currentUser === user) saving.value = false
  }
}

watch(() => authStore.currentUser, () => {
  keys.value = []
  credForm.apiKey = ''
  credForm.secret = ''
  saving.value = false
  void fetchKeys()
}, { flush: 'sync' })

onMounted(() => {
  loadEnvironment()
  fetchKeys()
})

defineExpose({
  saving,
  keys,
  credForm,
  fetchKeys,
  handleSaveCredential,
})
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

.api-key-card {
  margin-bottom: 20px;
}

.section-title {
  font-size: 14px;
  color: var(--color-text-secondary);
  margin-bottom: 12px;
}
</style>
