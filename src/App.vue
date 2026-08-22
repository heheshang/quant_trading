<template>
  <div id="app">
    <!-- Route navigation progress bar -->
    <div v-if="navigationLoading" class="route-progress"></div>
    <el-container class="layout-container">
      <!-- Fixed left sidebar, collapsible to an icon rail -->
      <el-aside :width="sidebarWidth" class="sidebar" v-if="auth.isAuthenticated">
        <div class="logo">
          <span class="logo-mark">Q</span>
          <div class="logo-text" v-show="!isCollapse">
            <span class="logo-title">量化交易系统</span>
            <span class="logo-sub">Quant Trading</span>
          </div>
        </div>
        <el-menu
          :default-active="$route.path"
          :collapse="isCollapse"
          :collapse-transition="false"
          router
          class="sidebar-menu"
          background-color="var(--color-bg-sidebar)"
          text-color="var(--color-text-sidebar)"
          active-text-color="var(--color-text-sidebar-active)"
        >
          <el-menu-item index="/dashboard" @mouseenter="prefetch('/dashboard')">
            <el-icon><DataLine /></el-icon>
            <template #title><span>仪表盘</span></template>
          </el-menu-item>
          <el-menu-item index="/strategy" @mouseenter="prefetch('/strategy')">
            <el-icon><Operation /></el-icon>
            <template #title><span>策略管理</span></template>
          </el-menu-item>
          <el-menu-item index="/backtest" @mouseenter="prefetch('/backtest')">
            <el-icon><TrendCharts /></el-icon>
            <template #title><span>回测系统</span></template>
          </el-menu-item>
          <el-menu-item index="/trading" @mouseenter="prefetch('/trading')">
            <el-icon><Sell /></el-icon>
            <template #title><span>交易执行</span></template>
          </el-menu-item>
          <el-menu-item index="/risk" @mouseenter="prefetch('/risk')">
            <el-icon><Warning /></el-icon>
            <template #title><span>风险管理</span></template>
          </el-menu-item>
          <el-menu-item index="/monitor" @mouseenter="prefetch('/monitor')">
            <el-icon><Monitor /></el-icon>
            <template #title><span>实时监控</span></template>
          </el-menu-item>
          <el-menu-item index="/settings" @mouseenter="prefetch('/settings')">
            <el-icon><Setting /></el-icon>
            <template #title><span>系统设置</span></template>
          </el-menu-item>
          <el-menu-item index="/profile" @mouseenter="prefetch('/profile')">
            <el-icon><User /></el-icon>
            <template #title><span>个人账户</span></template>
          </el-menu-item>
          <el-menu-item index="/binance" @mouseenter="prefetch('/binance')">
            <el-icon><Sell /></el-icon>
            <template #title><span>币安交易</span></template>
          </el-menu-item>
          <el-menu-item index="/test" @mouseenter="prefetch('/test')">
            <el-icon><Setting /></el-icon>
            <template #title><span>测试页面</span></template>
          </el-menu-item>
        </el-menu>
      </el-aside>

        <el-container>
        <!-- Show header only when authenticated -->
        <el-header class="header" v-if="auth.isAuthenticated">
          <div class="header-content">
            <div class="header-left">
              <el-button text circle class="menu-toggle" @click="toggleCollapse">
                <el-icon><MenuIcon /></el-icon>
              </el-button>
              <div class="header-titles">
                <h3>{{ pageTitle }}</h3>
                <el-breadcrumb separator="/" class="breadcrumb">
                  <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
                  <el-breadcrumb-item v-if="route.path !== '/dashboard'" :to="{ path: '/dashboard' }">仪表盘</el-breadcrumb-item>
                  <el-breadcrumb-item v-if="route.path !== '/dashboard'">{{ pageTitle }}</el-breadcrumb-item>
                </el-breadcrumb>
              </div>
            </div>
            <div class="user-info">
              <el-button class="theme-toggle" text circle @click="toggleTheme" :title="theme === 'dark' ? '切换亮色' : '切换暗色'">
                <el-icon><component :is="theme === 'dark' ? Sunny : Moon" /></el-icon>
              </el-button>
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
import { computed, onMounted, onBeforeUnmount, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Menu as MenuIcon, Moon, Sunny } from '@element-plus/icons-vue';
import { useAuthStore } from '@/stores/auth';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

// Fixed left sidebar that can collapse to an icon rail (stays on the left),
// remembers the preference, and auto-collapses on narrow screens.
const COLLAPSE_KEY = 'sidebarCollapse';
const isCollapse = ref(localStorage.getItem(COLLAPSE_KEY) === 'true');
const sidebarWidth = computed(() => (isCollapse.value ? '64px' : '200px'));

function toggleCollapse() {
  isCollapse.value = !isCollapse.value;
  localStorage.setItem(COLLAPSE_KEY, String(isCollapse.value));
}

function handleSidebarResize() {
  // Auto-collapse on small screens; restore saved preference when there's room.
  if (window.innerWidth < 768) {
    isCollapse.value = true;
  } else if (localStorage.getItem(COLLAPSE_KEY) !== null) {
    isCollapse.value = localStorage.getItem(COLLAPSE_KEY) === 'true';
  }
}

onMounted(() => {
  handleSidebarResize();
  window.addEventListener('resize', handleSidebarResize);
});
onBeforeUnmount(() => window.removeEventListener('resize', handleSidebarResize));

// Route-level loading indicator: show a top progress bar while the target
// lazy chunk resolves, so slow first-loads feel responsive.
const navigationLoading = ref(false);
router.beforeEach((_to, _from, next) => {
  navigationLoading.value = true;
  next();
});
router.afterEach(() => {
  navigationLoading.value = false;
});
router.onError(() => {
  navigationLoading.value = false;
});

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
// keep their data fresh via internal polling/WebSocket and refresh on
// `onActivated`. Component names come from the SFC `<script setup>` filename.
const keepAliveInclude = ['Dashboard', 'Strategy', 'Monitor', 'Trading'];

// ── Theme (light / dark), persisted — Element Plus dark css-vars handle ep components. ──
const theme = ref<'light' | 'dark'>(
  localStorage.getItem('theme') === 'dark' ? 'dark' : 'light',
);
function applyTheme(t: 'light' | 'dark') {
  document.documentElement.classList.toggle('dark', t === 'dark');
  localStorage.setItem('theme', t);
}
applyTheme(theme.value);
function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark';
  applyTheme(theme.value);
}

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
  background: var(--color-bg-sidebar);
  height: 100%;
  transition: width var(--transition-normal);
  overflow: hidden;
}

.sidebar-menu {
  border-right: none;
  height: calc(100% - var(--logo-height));
  overflow-y: auto;
}

.sidebar-menu:not(.el-menu--collapse) {
  width: 200px;
}

.menu-toggle {
  color: var(--color-text-regular);
  margin-right: var(--space-xs);
}

.logo {
  height: var(--logo-height);
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: 0 var(--space-md);
  color: #fff;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  white-space: nowrap;
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

.theme-toggle {
  color: var(--color-text-secondary);
  margin-right: var(--space-xs);
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
  align-items: center;
  gap: var(--space-sm);
}

.header-titles {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.header-titles h3 {
  margin: 0;
}

.breadcrumb {
  font-size: var(--font-size-xs);
}

.breadcrumb .el-breadcrumb__inner {
  font-size: var(--font-size-xs);
}

/* Route navigation progress bar — indeterminate top loader */
.route-progress {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 2px;
  z-index: 3000;
  pointer-events: none;
  background: linear-gradient(90deg, var(--color-primary), var(--chart-teal));
  transform-origin: left;
  animation: route-progress-slide 1s ease-in-out infinite;
}

@keyframes route-progress-slide {
  0% {
    transform: scaleX(0);
    transform-origin: left;
  }
  50% {
    transform: scaleX(1);
    transform-origin: left;
  }
  100% {
    transform: scaleX(0);
    transform-origin: right;
  }
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

/* ── Mobile / narrow-screen layout tuning ── */
@media (max-width: 768px) {
  .main-content {
    padding: var(--space-md);
  }
  .header {
    padding: 0 var(--space-sm);
  }
  .breadcrumb {
    display: none;
  }
  .user-name {
    display: none;
  }
  .header-content h3 {
    font-size: var(--font-size-md);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
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
  background: var(--color-text-placeholder);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-secondary);
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

/* ── Mobile / narrow-screen component adaptation ── */
@media (max-width: 768px) {
  .el-dialog {
    --el-dialog-width: 92%;
    width: 92%;
  }
  .el-message-box {
    width: 90%;
  }
  .el-col {
    max-width: 100% !important;
    flex: 0 0 100% !important;
  }
  .el-row + .el-row {
    margin-top: 12px;
  }
  .el-table {
    font-size: 12px;
  }
  .el-form-item__label {
    width: 84px !important;
  }
  .el-card__body {
    padding: 12px;
  }
  .el-card__header {
    padding: 12px 14px;
  }
}
</style>