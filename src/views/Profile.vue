<template>
  <div class="profile-container">
    <el-row :gutter="20">
      <el-col :span="24">
        <h2>个人账户信息</h2>
      </el-col>
    </el-row>

    <el-row :gutter="20">
      <!-- Account Information Card -->
      <el-col :span="16">
        <el-card class="account-info-card">
          <template #header>
            <div class="card-header">
              <span>账户信息</span>
              <el-button 
                v-if="!isEditing" 
                type="primary" 
                @click="startEdit"
                :loading="loading"
              >
                编辑信息
              </el-button>
              <div v-else>
                <el-button @click="cancelEdit">取消</el-button>
                <el-button type="primary" @click="saveProfile" :loading="saving">保存</el-button>
              </div>
            </div>
          </template>

          <el-form 
            :model="profileForm" 
            :rules="profileRules" 
            ref="profileFormRef" 
            label-width="120px"
            :disabled="!isEditing"
          >
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="账户ID" prop="account_id">
                  <el-input v-model="profileForm.account_id" readonly />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="用户名" prop="username">
                  <el-input v-model="profileForm.username" />
                </el-form-item>
              </el-col>
            </el-row>

            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="邮箱" prop="email">
                  <el-input v-model="profileForm.email" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="手机号" prop="phone">
                  <el-input v-model="profileForm.phone" />
                </el-form-item>
              </el-col>
            </el-row>

            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="姓名" prop="full_name">
                  <el-input v-model="profileForm.full_name" />
                </el-form-item>
              </el-col>
              
              <el-col :span="12">
                <el-form-item label="公司" prop="company">
                  <el-input v-model="profileForm.company" />
                </el-form-item>
              </el-col>
            </el-row>

            <el-row :gutter="20">
              <el-col :span="24">
                <el-form-item label="地址" prop="address">
                  <el-input v-model="profileForm.address" type="textarea" />
                </el-form-item>
              </el-col>
            </el-row>
          </el-form>
        </el-card>
      </el-col>

      <!-- Account Summary Card -->
      <el-col :span="8">
        <el-card class="account-summary-card">
          <template #header>
            <div class="card-header">
              <span>账户概览</span>
            </div>
          </template>

          <div class="account-summary">
            <div class="summary-item">
              <div class="summary-label">总资产</div>
              <div class="summary-value">¥{{ formatCurrency(accountInfo.total_assets) }}</div>
            </div>
            
            <div class="summary-item">
              <div class="summary-label">可用资金</div>
              <div class="summary-value">¥{{ formatCurrency(accountInfo.available_cash) }}</div>
            </div>
            
            <div class="summary-item">
              <div class="summary-label">持仓市值</div>
              <div class="summary-value">¥{{ formatCurrency(accountInfo.market_value) }}</div>
            </div>
            
            <div class="summary-item">
              <div class="summary-label">当日盈亏</div>
              <div 
                class="summary-value" 
                :class="{ positive: accountInfo.daily_pnl > 0, negative: accountInfo.daily_pnl < 0 }"
              >
                ¥{{ formatCurrency(accountInfo.daily_pnl) }}
              </div>
            </div>
            
            <div class="summary-item">
              <div class="summary-label">保证金比例</div>
              <div class="summary-value">{{ (accountInfo.margin_ratio * 100).toFixed(2) }}%</div>
            </div>
            
            <div class="summary-item">
              <div class="summary-label">更新时间</div>
              <div class="summary-value">{{ formatDate(accountInfo.updated_at) }}</div>
            </div>
          </div>
        </el-card>

        <!-- Security Settings Card -->
        <el-card class="security-card" style="margin-top: 20px;">
          <template #header>
            <div class="card-header">
              <span>安全设置</span>
            </div>
          </template>

          <div class="security-settings">
            <el-button type="primary" @click="showPasswordDialog = true" style="width: 100%; margin-bottom: 10px;">
              修改密码
            </el-button>
            <el-button @click="show2FADialog = true" style="width: 100%;">
              双因素认证
            </el-button>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- Change Password Dialog -->
    <el-dialog v-model="showPasswordDialog" title="修改密码" width="500px">
      <el-form :model="passwordForm" :rules="passwordRules" ref="passwordFormRef" label-width="100px">
        <el-form-item label="当前密码" prop="currentPassword">
          <el-input v-model="passwordForm.currentPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="新密码" prop="newPassword">
          <el-input v-model="passwordForm.newPassword" type="password" show-password />
        </el-form-item>
        <el-form-item label="确认密码" prop="confirmPassword">
          <el-input v-model="passwordForm.confirmPassword" type="password" show-password />
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="showPasswordDialog = false">取消</el-button>
          <el-button type="primary" @click="changePassword" :loading="changingPassword">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 2FA Dialog -->
    <el-dialog v-model="show2FADialog" title="双因素认证" width="500px">
      <div v-if="!is2FAEnabled">
        <p>启用双因素认证以增强账户安全性</p>
        <el-button type="primary" @click="enable2FA">启用 2FA</el-button>
      </div>
      <div v-else>
        <p>双因素认证已启用</p>
        <el-button type="danger" @click="disable2FA">禁用 2FA</el-button>
      </div>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="show2FADialog = false">关闭</el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage, ElMessageBox, FormInstance } from 'element-plus';

// Reactive data
const profileForm = ref({
  account_id: '',
  username: '',
  email: '',
  phone: '',
  full_name: '',
  company: '',
  address: ''
});

const accountInfo = ref({
  account_id: '',
  total_assets: 0,
  available_cash: 0,
  frozen_cash: 0,
  market_value: 0,
  total_pnl: 0,
  daily_pnl: 0,
  margin: 0,
  margin_ratio: 0,
  updated_at: new Date().toISOString()
});

const passwordForm = ref({
  currentPassword: '',
  newPassword: '',
  confirmPassword: ''
});

const profileFormRef = ref<FormInstance>();
const passwordFormRef = ref<FormInstance>();

const isEditing = ref(false);
const loading = ref(false);
const saving = ref(false);
const changingPassword = ref(false);
const showPasswordDialog = ref(false);
const show2FADialog = ref(false);
const is2FAEnabled = ref(false);

// Validation rules
const profileRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '用户名长度应在3-20个字符之间', trigger: 'blur' }
  ],
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入正确的邮箱地址', trigger: 'blur' }
  ],
  phone: [
    { required: true, message: '请输入手机号', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' }
  ]
};

const passwordRules = {
  currentPassword: [
    { required: true, message: '请输入当前密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6位', trigger: 'blur' }
  ],
  newPassword: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '密码长度至少6位', trigger: 'blur' }
  ],
  confirmPassword: [
    { required: true, message: '请确认新密码', trigger: 'blur' },
    { 
      validator: (rule: any, value: string, callback: any) => {
        if (value !== passwordForm.value.newPassword) {
          callback(new Error('两次输入的密码不一致'));
        } else {
          callback();
        }
      },
      trigger: 'blur'
    }
  ]
};

// Format currency
function formatCurrency(value: any): string {
  if (!value) return '0.00';
  return parseFloat(value.toString()).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}

// Format date
function formatDate(date: string): string {
  return new Date(date).toLocaleString('zh-CN');
}

// Start editing profile
function startEdit() {
  isEditing.value = true;
}

// Cancel editing
function cancelEdit() {
  isEditing.value = false;
  // Reset form to original values
  fetchProfile();
}

// Save profile
async function saveProfile() {
  if (!profileFormRef.value) return;
  
  await profileFormRef.value.validate(async (valid) => {
    if (!valid) return;
    
    saving.value = true;
    try {
      // Call Tauri command to update profile
      const result = await invoke<boolean>('update_profile', { 
        profileData: profileForm.value 
      });
      
      if (result) {
        ElMessage.success('个人信息保存成功');
        isEditing.value = false;
      } else {
        ElMessage.error('保存失败');
      }
    } catch (error) {
      console.error('Failed to save profile:', error);
      ElMessage.error('保存失败: ' + (error as Error).message);
    } finally {
      saving.value = false;
    }
  });
}

// Change password
async function changePassword() {
  if (!passwordFormRef.value) return;
  
  await passwordFormRef.value.validate(async (valid) => {
    if (!valid) return;
    
    changingPassword.value = true;
    try {
      // Call Tauri command to change password
      const result = await invoke<boolean>('change_password', {
        currentPassword: passwordForm.value.currentPassword,
        newPassword: passwordForm.value.newPassword
      });
      
      if (result) {
        ElMessage.success('密码修改成功');
        showPasswordDialog.value = false;
        // Reset password form
        passwordForm.value = {
          currentPassword: '',
          newPassword: '',
          confirmPassword: ''
        };
      } else {
        ElMessage.error('密码修改失败');
      }
    } catch (error) {
      console.error('Failed to change password:', error);
      ElMessage.error('密码修改失败: ' + (error as Error).message);
    } finally {
      changingPassword.value = false;
    }
  });
}

// Enable 2FA
async function enable2FA() {
  try {
    // In a real implementation, this would call a Tauri command to enable 2FA
    await new Promise(resolve => setTimeout(resolve, 1000)); // Simulate API call
    
    is2FAEnabled.value = true;
    ElMessage.success('双因素认证已启用');
  } catch (error) {
    console.error('Failed to enable 2FA:', error);
    ElMessage.error('启用双因素认证失败: ' + (error as Error).message);
  }
}

// Disable 2FA
async function disable2FA() {
  try {
    ElMessageBox.confirm('确定要禁用双因素认证吗？这会降低账户安全性。', '确认禁用', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    }).then(async () => {
      // In a real implementation, this would call a Tauri command to disable 2FA
      await new Promise(resolve => setTimeout(resolve, 1000)); // Simulate API call
      
      is2FAEnabled.value = false;
      ElMessage.success('双因素认证已禁用');
      show2FADialog.value = false;
    }).catch(() => {
      // User cancelled
    });
  } catch (error) {
    console.error('Failed to disable 2FA:', error);
    ElMessage.error('禁用双因素认证失败: ' + (error as Error).message);
  }
}

// Fetch profile data
async function fetchProfile() {
  loading.value = true;
  try {
    // Fetch account info
    const accountData = await invoke<any>('get_account_info');
    accountInfo.value = accountData;
    
    // Fetch user profile
    const userProfile = await invoke<any>('get_user_profile');
    profileForm.value = {
      account_id: accountData.account_id,
      username: userProfile.username,
      email: userProfile.email,
      phone: userProfile.phone,
      full_name: userProfile.full_name,
      company: userProfile.company,
      address: userProfile.address
    };
  } catch (error) {
    console.error('Failed to fetch profile:', error);
    ElMessage.error('获取个人信息失败');
  } finally {
    loading.value = false;
  }
}

// Initialize on mount
onMounted(() => {
  fetchProfile();
});
</script>

<style scoped>
.profile-container {
  padding: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.account-info-card {
  margin-bottom: 20px;
}

.account-summary {
  padding: 20px 0;
}

.summary-item {
  display: flex;
  justify-content: space-between;
  margin-bottom: 15px;
  padding-bottom: 15px;
  border-bottom: 1px solid #eee;
}

.summary-item:last-child {
  margin-bottom: 0;
  padding-bottom: 0;
  border-bottom: none;
}

.summary-label {
  font-size: 14px;
  color: #666;
}

.summary-value {
  font-size: 16px;
  font-weight: bold;
  color: #333;
}

.summary-value.positive {
  color: #67C23A;
}

.summary-value.negative {
  color: #F56C6C;
}

.security-settings {
  padding: 20px 0;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>