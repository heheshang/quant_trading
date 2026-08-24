<template>
  <div id="app">
    <!-- Route navigation progress bar -->
    <div v-if="navigationLoading" class="route-progress"></div>
    <el-container class="layout-container">
      <!-- Desktop: fixed left sidebar, collapsible to an icon rail -->
      <el-aside :width="sidebarWidth" class="sidebar" v-if="auth.isAuthenticated && !isMobile">
        <SidebarNav :collapse="isCollapse" />
      </el-aside>

        <el-container>
        <!-- Show header only when authenticated -->
        <el-header class="header" v-if="auth.isAuthenticated">
          <div class="header-content">
            <div class="header-left">
              <el-button text circle class="menu-toggle" @click="onMenuClick">
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

    <!-- Mobile: slide-in navigation drawer -->
    <el-drawer
      v-if="auth.isAuthenticated && isMobile"
      v-model="drawerVisible"
      class="nav-drawer"
      direction="ltr"
      size="220px"
      :with-header="false"
    >
      <SidebarNav :collapse="false" @select="drawerVisible = false" />
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Menu as MenuIcon, Moon, Sunny } from '@element-plus/icons-vue';
import { useAuthStore } from '@/stores/auth';
import SidebarNav from '@/components/layout/SidebarNav.vue';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

// Navigation structure: desktop = fixed collapsible sidebar; mobile = drawer.
const COLLAPSE_KEY = 'sidebarCollapse';
const isCollapse = ref(localStorage.getItem(COLLAPSE_KEY) === 'true');
const sidebarWidth = computed(() => (isCollapse.value ? '64px' : '200px'));
const isMobile = ref(window.innerWidth < 768);
const drawerVisible = ref(false);

function toggleCollapse() {
  isCollapse.value = !isCollapse.value;
  localStorage.setItem(COLLAPSE_KEY, String(isCollapse.value));
}

function onMenuClick() {
  if (isMobile.value) drawerVisible.value = true;
  else toggleCollapse();
}

function handleSidebarResize() {
  isMobile.value = window.innerWidth < 768;
  // Auto-collapse the fixed sidebar on narrow screens when there's room;
  // restore the saved preference on desktop.
  if (isMobile.value) {
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
    await auth.setRedirectPath(route.path);
    await router.push('/login');
  }
});

// Keep auth state in sync across route changes
router.afterEach((to) => {
  void auth.restoreSession().then(async (isValidSession) => {
    if (!isValidSession && to.path !== '/login') {
      await auth.setRedirectPath(to.path);
      void router.push('/login');
    }
  });
});

// Logout
const logout = () => {
  void auth.clearSession();
  void router.push('/login');
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

.nav-drawer :deep(.el-drawer__body) {
  background: var(--color-bg-sidebar);
  padding: 0;
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
  body {
    overflow-x: hidden;
  }
  .el-dialog {
    --el-dialog-width: 92%;
    width: 92%;
  }
  .el-message-box {
    width: 90%;
  }
  /* Per-page responsive breakpoints handle el-col layout; spacing is
     applied here so rows/cards breathe without forcing full-width cols. */
  .el-row + .el-row {
    margin-top: var(--space-md);
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

  /* Wide tables scroll horizontally inside their card instead of cropping. */
  .el-table .el-scrollbar__wrap {
    overflow-x: auto;
  }
  .el-table__body-wrapper,
  .el-table__header-wrapper {
    overflow-x: auto;
  }

  /* ── Human-friendly mobile ergonomics ── */
  body {
    line-height: 1.55;
    font-size: 14px;
  }
  .main-content {
    padding: var(--space-sm) var(--space-md) var(--space-xl);
  }
  /* Breathing room between stacked cards */
  .el-card {
    margin-bottom: var(--space-md);
  }
  .el-row + .el-row {
    margin-top: var(--space-md);
  }
  /* Comfortable touch targets (>= ~40px) */
  .el-button {
    min-height: 40px;
    padding: 10px 14px;
  }
  .el-input__inner,
  .el-textarea__inner,
  .el-select__wrapper {
    min-height: 40px !important;
    font-size: 14px;
  }
  .el-input--small .el-input__inner {
    height: 40px;
  }
  .el-form-item {
    margin-bottom: var(--space-md);
  }
  .el-table {
    line-height: 1.5;
  }
  /* Readable labels/values */
  .stat-value {
    font-size: 22px;
  }
}
</style>