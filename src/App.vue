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
            <span class="logo-mark">Q</span>
            <div class="logo-text">
              <span class="logo-title">量化交易系统</span>
              <span class="logo-sub">Quant Trading</span>
            </div>
          </div>
          
          <el-menu-item index="/dashboard" @mouseenter="prefetch('/dashboard')">
            <el-icon><DataLine /></el-icon>
            <span>仪表盘</span>
          </el-menu-item>
          
          <el-menu-item index="/strategy" @mouseenter="prefetch('/strategy')">
            <el-icon><Operation /></el-icon>
            <span>策略管理</span>
          </el-menu-item>
          
          <el-menu-item index="/backtest" @mouseenter="prefetch('/backtest')">
            <el-icon><TrendCharts /></el-icon>
            <span>回测系统</span>
          </el-menu-item>
          
          <el-menu-item index="/trading" @mouseenter="prefetch('/trading')">
            <el-icon><Sell /></el-icon>
            <span>交易执行</span>
          </el-menu-item>
          
          <el-menu-item index="/risk" @mouseenter="prefetch('/risk')">
            <el-icon><Warning /></el-icon>
            <span>风险管理</span>
          </el-menu-item>
          
          <el-menu-item index="/monitor" @mouseenter="prefetch('/monitor')">
            <el-icon><Monitor /></el-icon>
            <span>实时监控</span>
          </el-menu-item>
          
          <el-menu-item index="/settings" @mouseenter="prefetch('/settings')">
            <el-icon><Setting /></el-icon>
            <span>系统设置</span>
          </el-menu-item>
          
          <el-menu-item index="/profile" @mouseenter="prefetch('/profile')">
            <el-icon><User /></el-icon>
            <span>个人账户</span>
          </el-menu-item>
          
          <el-menu-item index="/binance" @mouseenter="prefetch('/binance')">
            <el-icon><Sell /></el-icon>
            <span>币安交易</span>
          </el-menu-item>

          <el-menu-item index="/test" @mouseenter="prefetch('/test')">
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
              <span class="user-avatar">{{ auth.username?.charAt(0)?.toUpperCase() || 'U' }}</span>
              <span class="user-name">{{ auth.username }}</span>
              <el-button type="text" class="logout-btn" @click="logout">退出</el-button>
            </div>
          </div>
        </el-header>
        
        <el-main class="main-content">
          <router-view v-slot="{ Component }">
            <transition name="fade-slide" mode="out-in">
              <keep-alive :include="keepAliveInclude" :max="10">
                <component :is="Component" />
              </keep-alive>
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

// Prefetch lazy route components on hover so page switches feel instant.
const routePreloaders: Record<string, () => Promise<unknown>> = {
  '/dashboard': () => import('@/views/Dashboard.vue'),
  '/strategy': () => import('@/views/Strategy.vue'),
  '/backtest': () => import('@/views/Backtest.vue'),
  '/trading': () => import('@/views/Trading.vue'),
  '/risk': () => import('@/views/Risk.vue'),
  '/monitor': () => import('@/views/Monitor.vue'),
  '/settings': () => import('@/views/Settings.vue'),
  '/profile': () => import('@/views/Profile.vue'),
  '/binance': () => import('@/views/Binance.vue'),
  '/test': () => import('@/views/Test.vue'),
};

const prefetch = (path: string) => {
  routePreloaders[path]?.();
};

// Cache frequently-visited pages so switching back is instant; these views
// keep their data fresh via internal polling/WebSocket (no stale data).
// Component names come from the SFC `<script setup>` filename.
const keepAliveInclude = ['Dashboard', 'Strategy'];

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
    '/binance': '币安交易',
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
  background-color: var(--color-bg-sidebar);
}

.sidebar-menu {
  border: none;
  height: 100%;
}

.logo {
  height: var(--logo-height);
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: 0 var(--space-md);
  color: #fff;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.logo-mark {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: linear-gradient(135deg, var(--color-primary), var(--chart-teal));
  color: #fff;
  font-size: 18px;
  font-weight: 700;
  box-shadow: var(--shadow-sm);
}

.logo-text {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.logo-title {
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--color-text-sidebar);
}

.logo-sub {
  font-size: var(--font-size-xs);
  color: rgba(191, 203, 217, 0.6);
}

.header {
  background: var(--color-bg-white);
  box-shadow: var(--shadow-md);
  padding: 0 var(--space-xl);
  z-index: 1;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 100%;
}

.header-content h3 {
  margin: 0;
  font-size: var(--font-size-lg);
  color: var(--color-text-primary);
  font-weight: 600;
}

.user-info {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}

.user-avatar {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--color-primary);
  color: #fff;
  font-size: var(--font-size-md);
  font-weight: 600;
}

.user-name {
  font-size: var(--font-size-sm);
  color: var(--color-text-regular);
}

.logout-btn {
  color: var(--color-text-secondary);
}

.main-content {
  background: var(--color-bg-page);
  padding: var(--space-xl);
  overflow-y: auto;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.breadcrumb {
  font-size: var(--font-size-xs);
}

.breadcrumb .el-breadcrumb__inner {
  font-size: var(--font-size-xs);
}

/* Route transition — snappy, GPU-accelerated fade + slight lift */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
  will-change: opacity, transform;
  backface-visibility: hidden;
}

.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body {
  height: 100%;
  background: var(--color-bg-page);
}

#app {
  font-family: var(--font-family);
  color: var(--color-text-primary);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* Smooth, theme-aware scrollbar */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: #c0c4cc;
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: #909399;
}

/* ── Global component polish (theme-aware) ── */
.el-card {
  border-radius: var(--radius-lg);
  border-color: var(--color-border-light);
  box-shadow: var(--shadow-sm);
}

.el-card__header {
  font-weight: 600;
  color: var(--color-text-primary);
  border-bottom: 1px solid var(--color-border-light);
}

.el-table {
  --el-table-border-color: var(--color-border-light);
  --el-table-header-bg-color: #f5f7fa;
  font-size: var(--font-size-sm);
}

.el-table th.el-table__cell {
  color: var(--color-text-regular);
  font-weight: 600;
}

.el-button {
  border-radius: var(--radius-sm);
}

.el-dialog {
  border-radius: var(--radius-lg);
}
</style>