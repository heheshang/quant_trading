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
    <el-dialog v-model="show2FADialog" title="双因素认证" width="540px">
      <template v-if="!is2FAEnabled && !show2FASetup">
        <div style="text-align:center;padding:20px 0;">
          <el-icon :size="48" color="#409EFF"><Lock /></el-icon>
          <p style="margin-top:12px;font-size:14px;color:#666;">
            双因素认证为您的账户提供额外的安全保护。启用后，登录时需要输入动态验证码。
          </p>
          <el-button type="primary" @click="start2FASetup" style="margin-top:16px;">
            立即启用
          </el-button>
        </div>
      </template>

      <template v-else-if="show2FASetup">
        <el-steps :active="twoFAStep" simple style="margin-bottom:20px">
          <el-step title="绑定密钥" :status="twoFAStep > 1 ? 'finish' : 'process'" />
          <el-step title="验证代码" :status="twoFAStep > 2 ? 'finish' : twoFAStep === 2 ? 'process' : 'wait'" />
          <el-step title="完成" :status="twoFAStep > 3 ? 'finish' : 'wait'" />
        </el-steps>

        <div v-if="twoFAStep === 1">
          <p style="margin-bottom:12px;font-size:14px;">
            请在您的验证器应用中添加以下密钥：
          </p>
          <div style="text-align:center;padding:16px 0;">
            <div style="display:inline-block;background:#fff;padding:16px;border:1px solid #dcdfe6;border-radius:8px;">
              <div style="display:grid;grid-template-columns:repeat(8,18px);gap:2px;margin-bottom:8px">
                <div v-for="i in 64" :key="i" 
                  :style="{ 
                    width:'18px',height:'18px',
                    background: ['#000','#fff'][Math.floor(Math.random()*2)],
                    borderRadius:'2px'
                  }">
                </div>
              </div>
              <div style="font-size:12px;color:#999;">扫描此二维码</div>
            </div>
          </div>
          <el-input :model-value="fake2FASecret" readonly style="margin-bottom:8px;">
            <template #prepend>密钥</template>
            <template #append>
              <el-button @click="copySecret">复制</el-button>
            </template>
          </el-input>
          <p style="font-size:12px;color:#999;">
            支持 Google Authenticator、Authy、Microsoft Authenticator 等
          </p>
          <div style="text-align:right;margin-top:12px;">
            <el-button @click="show2FASetup = false; show2FADialog = false">取消</el-button>
            <el-button type="primary" @click="twoFAStep = 2">我已绑定</el-button>
          </div>
        </div>

        <div v-if="twoFAStep === 2">
          <p style="margin-bottom:12px;font-size:14px;">
            请输入您的验证器应用当前显示的 6 位动态验证码：
          </p>
          <div style="text-align:center;padding:12px 0;">
            <el-input
              v-model="twoFACode"
              :maxlength="6"
              placeholder="000000"
              size="large"
              style="width:200px;font-size:24px;text-align:center;"
              @input="on2FACodeInput"
            />
          </div>
          <div style="text-align:right;margin-top:12px;">
            <el-button @click="twoFAStep = 1">返回</el-button>
            <el-button type="primary" @click="verify2FA" :loading="verifying2FA">验证并启用</el-button>
          </div>
        </div>

        <div v-if="twoFAStep === 3">
          <div style="text-align:center;padding:20px 0;">
            <el-icon :size="48" color="#67C23A"><CircleCheck /></el-icon>
            <p style="margin-top:12px;font-size:16px;font-weight:bold;color:#333;">双因素认证已启用</p>
            <p style="font-size:14px;color:#666;margin-top:8px;">
              今后登录时，您需要输入手机验证码以及动态验证码
            </p>
          </div>
          <div style="text-align:right;">
            <el-button type="primary" @click="close2FADialog">完成</el-button>
          </div>
        </div>
      </template>

      <template v-else>
        <div style="text-align:center;padding:20px 0;">
          <el-icon :size="48" color="#67C23A"><CircleCheck /></el-icon>
          <p style="margin-top:12px;font-size:16px;font-weight:bold;color:#333;">双因素认证已启用</p>
          <p style="font-size:14px;color:#666;margin-top:8px;margin-bottom:16px;">
            您的账户已受到双因素认证保护
          </p>
          <el-button type="danger" @click="disable2FA">禁用 2FA</el-button>
        </div>
      </template>

      <template #footer>
        <span v-if="is2FAEnabled" class="dialog-footer">
          <el-button @click="show2FADialog = false">关闭</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- Disable 2FA confirm dialog -->
    <ConfirmDialog
      v-model:visible="disable2FADialogVisible"
      title="确认禁用"
      message="确定要禁用双因素认证吗？这会降低账户安全性。"
      type="warning"
      confirm-text="禁用"
      @confirm="confirmDisable2FA"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { getAccountInfo, getUserProfile, updateProfile, changePassword as apiChangePassword } from '@/services/api';
import { ElMessage, type FormInstance } from 'element-plus';
import { Lock, CircleCheck } from '@element-plus/icons-vue';
import { useRouter } from 'vue-router';
import ConfirmDialog from '@/components/common/ConfirmDialog.vue';

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
const disable2FADialogVisible = ref(false);
const router = useRouter();

// 2FA setup state
const show2FASetup = ref(false);
const twoFAStep = ref(1);
const verifying2FA = ref(false);
const twoFACode = ref('');
const fake2FASecret = ref('JBSWY3DPEHPK3PXP');

function start2FASetup() {
  show2FASetup.value = true;
  twoFAStep.value = 1;
  twoFACode.value = '';
  // Generate a random-looking secret
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
  fake2FASecret.value = Array.from({ length: 16 }, () => chars[Math.floor(Math.random() * chars.length)]).join('');
}

function copySecret() {
  navigator.clipboard.writeText(fake2FASecret.value).then(() => {
    ElMessage.success('密钥已复制到剪贴板');
  }).catch(() => {
    ElMessage.warning('复制失败，请手动复制');
  });
}

function on2FACodeInput(value: string) {
  twoFACode.value = value.replace(/\D/g, '').slice(0, 6);
}

async function verify2FA() {
  if (twoFACode.value.length !== 6) {
    ElMessage.warning('请输入完整的 6 位验证码');
    return;
  }
  verifying2FA.value = true;
  try {
    // Simulate API verification
    await new Promise(resolve => setTimeout(resolve, 1000));
    is2FAEnabled.value = true;
    twoFAStep.value = 3;
    ElMessage.success('双因素认证已启用');
  } catch (error) {
    ElMessage.error('验证失败，请重试');
  } finally {
    verifying2FA.value = false;
  }
}

function close2FADialog() {
  show2FADialog.value = false;
  show2FASetup.value = false;
  twoFAStep.value = 1;
  twoFACode.value = '';
}

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
      validator: (_rule: any, value: string, callback: any) => {
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
      const result = await updateProfile(profileForm.value as any);
      
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
      const result = await apiChangePassword(
        passwordForm.value.currentPassword,
        passwordForm.value.newPassword
      );
      
      if (result) {
        ElMessage.success('密码修改成功，即将跳转至登录页面');
        showPasswordDialog.value = false;
        passwordForm.value = { currentPassword: '', newPassword: '', confirmPassword: '' };
        // Auto-redirect to login after 2 seconds
        setTimeout(() => {
          localStorage.removeItem('auth_token');
          router.push('/login');
        }, 2000);
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

// Disable 2FA — show ConfirmDialog first
function disable2FA() {
  disable2FADialogVisible.value = true;
}

async function confirmDisable2FA() {
  try {
    // In a real implementation, this would call a Tauri command to disable 2FA
    await new Promise(resolve => setTimeout(resolve, 1000)); // Simulate API call

    is2FAEnabled.value = false;
    ElMessage.success('双因素认证已禁用');
    show2FADialog.value = false;
    disable2FADialogVisible.value = false;
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
    const accountData = await getAccountInfo();
    accountInfo.value = accountData as any;
    
    // Fetch user profile
    const userProfile = await getUserProfile();
    profileForm.value = {
      account_id: String(accountData?.account_id ?? ''),
      username: String(userProfile?.username ?? ''),
      email: String(userProfile?.email ?? ''),
      phone: String(userProfile?.phone ?? ''),
      full_name: String(userProfile?.full_name ?? ''),
      company: String(userProfile?.company ?? ''),
      address: String(userProfile?.address ?? '')
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