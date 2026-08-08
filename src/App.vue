<template>
  <div id="app">
    <el-container class="layout-container">
      <!-- Show sidebar only when authenticated -->
      <el-aside width="200px" class="sidebar" v-if="auth.isAuthenticated">
        <el-menu
          :default-active="$route.path"
          router
          class="sidebar-menu"
          background-color="#304156"
          text-color="#bfcbd9"
          active-text-color="#409EFF"
        >
          <div class="logo">
            <h2>量化交易系统</h2>
          </div>
          
          <el-menu-item index="/dashboard">
            <el-icon><DataLine /></el-icon>
            <span>仪表盘</span>
          </el-menu-item>
          
          <el-menu-item index="/strategy">
            <el-icon><Operation /></el-icon>
            <span>策略管理</span>
          </el-menu-item>
          
          <el-menu-item index="/backtest">
            <el-icon><TrendCharts /></el-icon>
            <span>回测系统</span>
          </el-menu-item>
          
          <el-menu-item index="/trading">
            <el-icon><Sell /></el-icon>
            <span>交易执行</span>
          </el-menu-item>
          
          <el-menu-item index="/risk">
            <el-icon><Warning /></el-icon>
            <span>风险管理</span>
          </el-menu-item>
          
          <el-menu-item index="/monitor">
            <el-icon><Monitor /></el-icon>
            <span>实时监控</span>
          </el-menu-item>
          
          <el-menu-item index="/settings">
            <el-icon><Setting /></el-icon>
            <span>系统设置</span>
          </el-menu-item>
          
          <el-menu-item index="/profile">
            <el-icon><User /></el-icon>
            <span>个人账户</span>
          </el-menu-item>
          
          <el-menu-item index="/test">
            <el-icon><Setting /></el-icon>
            <span>测试页面</span>
          </el-menu-item>
        </el-menu>
      </el-aside>
      
      <el-container>
        <!-- Show header only when authenticated -->
        <el-header class="header" v-if="auth.isAuthenticated">
          <div class="header-content">
            <div class="header-left">
              <h3>{{ pageTitle }}</h3>
              <el-breadcrumb separator="/" class="breadcrumb">
                <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
                <el-breadcrumb-item v-if="route.path !== '/dashboard'" :to="{ path: '/dashboard' }">仪表盘</el-breadcrumb-item>
                <el-breadcrumb-item v-if="route.path !== '/dashboard'">{{ pageTitle }}</el-breadcrumb-item>
              </el-breadcrumb>
            </div>
            <div class="user-info">
              <el-icon><User /></el-icon>
              <span>{{ auth.username }}</span>
              <el-button type="text" @click="logout">退出</el-button>
            </div>
          </div>
        </el-header>
        
        <el-main class="main-content">
          <router-view v-slot="{ Component, route: r }">
            <transition name="fade-slide" mode="out-in">
              <component :is="Component" :key="r.path" />
            </transition>
          </router-view>
        </el-main>
      </el-container>
    </el-container>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useAuthStore } from '@/stores/auth';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

const pageTitle = computed(() => {
  const titles: Record<string, string> = {
    '/dashboard': '仪表盘',
    '/strategy': '策略管理',
    '/backtest': '回测系统',
    '/trading': '交易执行',
    '/risk': '风险管理',
    '/monitor': '实时监控',
    '/settings': '系统设置',
    '/profile': '个人账户',
    '/test': '测试页面',
  };
  return titles[route.path] || '量化交易系统';
});

// Restore session on mount and guard route
onMounted(async () => {
  const isValidSession = await auth.restoreSession();
  if (!isValidSession && route.path !== '/login') {
    auth.setRedirectPath(route.path);
    await router.push('/login');
  }
});

// Keep auth state in sync across route changes
router.afterEach((to) => {
  void auth.restoreSession().then((isValidSession) => {
    if (!isValidSession && to.path !== '/login') {
      auth.setRedirectPath(to.path);
      void router.push('/login');
    }
  });
});

// Logout
const logout = () => {
  auth.clearSession();
  router.push('/login');
};
</script>

<style scoped>
.layout-container {
  height: 100vh;
}

.sidebar {
  background-color: #304156;
}

.sidebar-menu {
  border: none;
  height: 100%;
}

.logo {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 18px;
  font-weight: bold;
  border-bottom: 1px solid #1f2d3d;
}

.logo h2 {
  font-size: 16px;
  margin: 0;
}

.header {
  background: #fff;
  box-shadow: 0 1px 4px rgba(0,21,41,.08);
  padding: 0 20px;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 100%;
}

.header-content h3 {
  margin: 0;
  font-size: 18px;
  color: #333;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.main-content {
  background: #f0f2f5;
  padding: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.breadcrumb {
  font-size: 12px;
}

.breadcrumb .el-breadcrumb__inner {
  font-size: 12px;
}

/* Route transition animations */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.25s ease;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}
</style>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

#app {
  font-family: 'Helvetica Neue', Helvetica, 'PingFang SC', 'Hiragino Sans GB',
    'Microsoft YaHei', Arial, sans-serif;
}
</style>