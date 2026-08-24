<template>
  <div class="login-container">
    <div class="login-box">
      <div class="login-header">
        <h2>量化交易系统</h2>
        <p>登录到您的账户</p>
      </div>
      
      <el-form 
        :model="loginForm" 
        :rules="loginRules" 
        ref="loginFormRef"
        class="login-form"
        @submit.prevent="handleLogin"
      >
        <el-form-item prop="username">
          <el-input
            v-model="loginForm.username"
            placeholder="用户名"
            prefix-icon="User"
            size="large"
            clearable
          />
        </el-form-item>
        
        <el-form-item prop="password">
          <el-input
            v-model="loginForm.password"
            type="password"
            placeholder="密码"
            prefix-icon="Lock"
            size="large"
            show-password
            @keyup.enter="handleLogin"
          />
        </el-form-item>
        
        <el-form-item v-if="needs2FA" prop="code">
          <el-input
            v-model="loginForm.code"
            placeholder="请输入 2FA 动态验证码"
            prefix-icon="Key"
            size="large"
            maxlength="6"
            @keyup.enter="handleLogin"
          />
          <div class="twofa-hint">该账户已启用二次验证，请输入验证码完成登录</div>
        </el-form-item>
        
        <el-form-item>
          <el-checkbox v-model="loginForm.remember">记住我</el-checkbox>
        </el-form-item>
        
        <el-form-item>
          <el-button
            type="primary"
            size="large"
            class="login-button"
            :loading="auth.loading"
            @click="handleLogin"
          >
            登录
          </el-button>
        </el-form-item>
      </el-form>
      
      <div class="login-footer">
        <p>© 2025 量化交易系统. 保留所有权利.</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useAuthStore } from '@/stores/auth';
import { ElMessage, FormInstance } from 'element-plus';

const router = useRouter();
const auth = useAuthStore();
const loginFormRef = ref<FormInstance>();

const loginForm = reactive({
  username: '',
  password: '',
  remember: false,
  code: ''
});

const needs2FA = ref(false);

const loginRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '用户名长度应在3-20个字符之间', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, max: 30, message: '密码长度应在6-30个字符之间', trigger: 'blur' }
  ],
  code: [
    { required: true, message: '请输入 2FA 验证码', trigger: 'blur' }
  ]
};

async function handleLogin() {
  if (!loginFormRef.value) return;
  if (auth.loading) return;

  // When 2FA is required the code field is mandatory before resubmitting.
  if (needs2FA.value && !loginForm.code.trim()) {
    ElMessage.warning('请输入 2FA 动态验证码');
    return;
  }

  await loginFormRef.value.validate(async (valid) => {
    if (!valid) return;

    try {
      const redirectPath = await auth.login(
        loginForm.username,
        loginForm.password,
        loginForm.remember,
        needs2FA.value ? loginForm.code : undefined,
      );
      needs2FA.value = false;
      loginForm.code = '';
      ElMessage.success('登录成功');
      router.push(redirectPath);
    } catch (error) {
      const message = (error as Error).message || '';
      // A "2FA code required" error flips the form into its second factor
      // step, revealing the code input for resubmission.
      if (message.includes('2FA')) {
        needs2FA.value = true;
      }
      ElMessage.error('登录失败: ' + message);
    }
  });
}

onMounted(async () => {
  const isValidSession = await auth.restoreSession();
  if (isValidSession) {
    router.push('/dashboard');
    return;
  }

  const remembered = auth.getRememberedUsername();
  if (remembered) {
    loginForm.username = remembered;
    loginForm.remember = true;
  }
});
</script>

<style scoped>
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  background: var(--el-bg-color-page, #f5f7fa);
}

.login-box {
  width: 100%;
  max-width: 400px;
  padding: 40px;
  background: rgba(255, 255, 255, 0.95);
  border-radius: 10px;
  box-shadow: 0 15px 35px rgba(0, 0, 0, 0.1);
}

.login-header {
  text-align: center;
  margin-bottom: 30px;
}

.login-header h2 {
  font-size: 24px;
  color: var(--color-text-primary);
  margin-bottom: 10px;
}

.login-header p {
  color: var(--color-text-regular);
  font-size: 14px;
}

.login-form {
  margin-bottom: 20px;
}

.login-button {
  width: 100%;
}
.twofa-hint {
  font-size: 12px;
  color: var(--el-color-warning, #e6a23c);
  margin-top: 6px;
  line-height: 1.4;
}

.login-footer {
  text-align: center;
  color: var(--color-text-secondary);
  font-size: 12px;
}
</style>