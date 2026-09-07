<template>
  <div class="sidebar-nav">
    <div class="logo">
      <span class="logo-mark">Q</span>
      <div class="logo-text" v-show="!collapse">
        <span class="logo-title">量化交易系统</span>
        <span class="logo-sub">Quant Trading</span>
      </div>
    </div>
    <div v-if="!collapse" class="nav-caption">工作空间</div>
    <el-menu
      :default-active="$route.path"
      :collapse="collapse"
      :collapse-transition="false"
      router
      class="sidebar-menu"
      background-color="var(--color-bg-sidebar)"
      text-color="var(--color-text-sidebar)"
      active-text-color="var(--color-text-sidebar-active)"
      @select="emit('select')"
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
      <el-menu-item index="/binance" @mouseenter="prefetch('/binance')">
        <el-icon><Sell /></el-icon>
        <template #title><span>币安交易</span></template>
      </el-menu-item>
      <el-menu-item class="nav-section-start" index="/settings" @mouseenter="prefetch('/settings')">
        <el-icon><Setting /></el-icon>
        <template #title><span>系统设置</span></template>
      </el-menu-item>
      <el-menu-item index="/profile" @mouseenter="prefetch('/profile')">
        <el-icon><User /></el-icon>
        <template #title><span>个人账户</span></template>
      </el-menu-item>
      <el-menu-item index="/test" @mouseenter="prefetch('/test')">
        <el-icon><Setting /></el-icon>
        <template #title><span>测试页面</span></template>
      </el-menu-item>
    </el-menu>
    <div v-if="!collapse" class="nav-footer">
      <span>QUANT TRADING</span>
      <p>研究策略 · 审慎执行</p>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ collapse: boolean }>()
const emit = defineEmits<{ select: [] }>()

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
}
const prefetch = (path: string) => {
  routePreloaders[path]?.()
}
</script>

<style scoped>
.sidebar-nav {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.logo {
  height: var(--logo-height);
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: 0 19px;
  color: #fff;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  white-space: nowrap;
  flex-shrink: 0;
}

.logo-mark {
  width: 34px;
  height: 34px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: #247d76;
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
  color: #eef5f8;
}

.logo-sub {
  font-size: var(--font-size-xs);
  color: #94aebd;
}

.sidebar-menu {
  border-right: none;
  flex: 1;
  overflow-y: auto;
  padding: var(--space-xs);
}

.sidebar-menu :deep(.el-menu-item) {
  height: 46px;
  margin: 4px 0;
  border-radius: var(--radius-md);
}

.sidebar-menu :deep(.el-menu-item.is-active) {
  background: #24463f;
  font-weight: 600;
}

.sidebar-menu :deep(.el-menu-item:hover) {
  background: #223d49;
}

.sidebar-menu.el-menu--collapse { width: 72px; }
.sidebar-menu :deep(.nav-section-start) { margin-top: 24px; }
.nav-caption { padding: 24px 28px 8px; font-size: 11px; letter-spacing: 0.14em; color: #94aebd; }
.nav-footer { padding: 20px 24px; border-top: 1px solid #2a3d49; color: #94aebd; }
.nav-footer span { font-size: 10px; letter-spacing: 0.16em; }
.nav-footer p { margin-top: 6px; font-size: 12px; }
</style>
