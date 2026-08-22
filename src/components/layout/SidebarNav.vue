<template>
  <div class="sidebar-nav">
    <div class="logo">
      <span class="logo-mark">Q</span>
      <div class="logo-text" v-show="!collapse">
        <span class="logo-title">量化交易系统</span>
        <span class="logo-sub">Quant Trading</span>
      </div>
    </div>
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
  padding: 0 var(--space-md);
  color: #fff;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  white-space: nowrap;
  flex-shrink: 0;
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

.sidebar-menu {
  border-right: none;
  flex: 1;
  overflow-y: auto;
}
</style>
