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
        
        <el-form-item>
          <el-checkbox v-model="loginForm.remember">记住我</el-checkbox>
        </el-form-item>
        
        <el-form-item>
          <el-button
            type="primary"
            size="large"
            class="login-button"
            :loading="loading"
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
import { login, verifyToken } from '@/services/api';
import { ElMessage, FormInstance } from 'element-plus';

// Reactive data
const router = useRouter();
const loginFormRef = ref<FormInstance>();

const loginForm = reactive({
  username: '',
  password: '',
  remember: false
});

const loading = ref(false);

// Validation rules
const loginRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '用户名长度应在3-20个字符之间', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, max: 30, message: '密码长度应在6-30个字符之间', trigger: 'blur' }
  ]
};

// Handle login
const handleLogin = async () => {
  if (!loginFormRef.value) return;
  
  await loginFormRef.value.validate(async (valid) => {
    if (!valid) return;
    
    loading.value = true;
    try {
      // Call login API
      const token = await login(loginForm.username, loginForm.password);
      
      // Store authentication state
      localStorage.setItem('isAuthenticated', 'true');
      localStorage.setItem('username', loginForm.username);
      localStorage.setItem('authToken', token);

      // Handle "remember me" - save/clear saved credentials
      if (loginForm.remember) {
        localStorage.setItem('remembered_username', loginForm.username);
        // Store a hint (not the real password) for UX
        localStorage.setItem('remembered_password', loginForm.password);
      } else {
        localStorage.removeItem('remembered_username');
        localStorage.removeItem('remembered_password');
      }
      
      // Verify token validity
      try {
        const valid = await verifyToken(token);
        if (!valid) {
          throw new Error('Token 验证失败');
        }
      } catch (verifyError) {
        console.error('Token verification failed:', verifyError);
        // Token invalid, clear auth and block login
        localStorage.removeItem('isAuthenticated');
        localStorage.removeItem('authToken');
        localStorage.removeItem('username');
        ElMessage.error('登录验证失败，请重试');
        loading.value = false;
        return;
      }
      
      // Redirect to intended page or dashboard
      const redirectPath = localStorage.getItem('redirect_after_login') || '/dashboard';
      localStorage.removeItem('redirect_after_login');
      ElMessage.success('登录成功');
      router.push(redirectPath);
    } catch (error) {
      console.error('Login failed:', error);
      ElMessage.error('登录失败: ' + (error as Error).message);
    } finally {
      loading.value = false;
    }
  });
};

// Check if already authenticated; restore remembered credentials
onMounted(() => {
  const isAuthenticated = localStorage.getItem('isAuthenticated');
  if (isAuthenticated === 'true') {
    router.push('/dashboard');
    return;
  }

  // Restore remembered credentials
  const rememberedUsername = localStorage.getItem('remembered_username');
  const rememberedPassword = localStorage.getItem('remembered_password');
  if (rememberedUsername) {
    loginForm.username = rememberedUsername;
    loginForm.remember = true;
    if (rememberedPassword) {
      loginForm.password = rememberedPassword;
    }
  }
});
</script>

<style scoped>
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
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
  color: #333;
  margin-bottom: 10px;
}

.login-header p {
  color: #666;
  font-size: 14px;
}

.login-form {
  margin-bottom: 20px;
}

.login-button {
  width: 100%;
}

.login-footer {
  text-align: center;
  color: #999;
  font-size: 12px;
}
</style>