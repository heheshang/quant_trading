<template>
  <div class="profile-container">
    <el-row :gutter="20">
      <el-col :span="24">
        <h2>个人账户信息</h2>
      </el-col>
    </el-row>

    <el-row :gutter="20">
      <el-col :span="16">
        <ProfileInfo
          :initial-form="profileForm"
          :is-editing="isEditing"
          :saving="saving"
          :loading="loading"
          @start-edit="isEditing = true"
          @cancel-edit="handleCancelEdit"
          @save="handleSaveProfile"
        />
      </el-col>

      <el-col :span="8">
        <AccountSummary :account-info="accountInfo" />
        <SecuritySettings
          @change-password="showPasswordDialog = true"
          @setup-2fa="show2FADialog = true"
        />
      </el-col>
    </el-row>

    <PasswordChange v-model:visible="showPasswordDialog" />
    <TwoFactorAuth v-model:visible="show2FADialog" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { getAccountInfo, getUserProfile, updateProfile } from '@/services/api'
import type { AccountInfo } from '@/services/types'
import ProfileInfo from '@/components/profile/ProfileInfo.vue'
import type { ProfileFormData } from '@/components/profile/ProfileInfo.vue'
import AccountSummary from '@/components/profile/AccountSummary.vue'
import SecuritySettings from '@/components/profile/SecuritySettings.vue'
import PasswordChange from '@/components/profile/PasswordChange.vue'
import TwoFactorAuth from '@/components/profile/TwoFactorAuth.vue'

const profileForm = ref<ProfileFormData>({
  account_id: '',
  username: '',
  email: '',
  phone: '',
  full_name: '',
  company: '',
  address: '',
})

const accountInfo = ref<AccountInfo>({
  account_id: 0,
  total_assets: 0,
  available_cash: 0,
  frozen_cash: 0,
  market_value: 0,
  total_pnl: 0,
  daily_pnl: 0,
  margin: 0,
  margin_ratio: 0,
  updated_at: new Date().toISOString(),
})

const isEditing = ref(false)
const loading = ref(false)
const saving = ref(false)
const showPasswordDialog = ref(false)
const show2FADialog = ref(false)

async function fetchProfile() {
  loading.value = true
  try {
    const accountData = await getAccountInfo()
    accountInfo.value = accountData

    const username = localStorage.getItem('username') || undefined
    const userProfile = await getUserProfile(username)
    profileForm.value = {
      account_id: String(accountData?.account_id ?? ''),
      username: String(userProfile?.username ?? ''),
      email: String(userProfile?.email ?? ''),
      phone: String(userProfile?.phone ?? ''),
      full_name: String(userProfile?.full_name ?? ''),
      company: String(userProfile?.company ?? ''),
      address: String(userProfile?.address ?? ''),
    }
  } catch (error) {
    console.error('Failed to fetch profile:', error)
    ElMessage.error('获取个人信息失败')
  } finally {
    loading.value = false
  }
}

function handleCancelEdit() {
  isEditing.value = false
  fetchProfile()
}

async function handleSaveProfile(formData: ProfileFormData) {
  saving.value = true
  try {
    const result = await updateProfile(formData as unknown as Record<string, unknown>)
    if (result) {
      ElMessage.success('个人信息保存成功')
      isEditing.value = false
    } else {
      ElMessage.error('保存失败')
    }
  } catch (error) {
    console.error('Failed to save profile:', error)
    ElMessage.error('保存失败: ' + (error as Error).message)
  } finally {
    saving.value = false
  }
}

onMounted(() => {
  fetchProfile()
})
</script>

<style scoped>
.profile-container {
  padding: 20px;
}
</style>
