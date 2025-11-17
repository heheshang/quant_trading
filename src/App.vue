<template>
  <div id="app">
    <el-container class="layout-container">
      <el-aside width="200px" class="sidebar">
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
          
          <el-menu-item index="/test">
            <el-icon><Setting /></el-icon>
            <span>测试页面</span>
          </el-menu-item>
        </el-menu>
      </el-aside>
      
      <el-container>
        <el-header class="header">
          <div class="header-content">
            <h3>{{ pageTitle }}</h3>
            <div class="user-info">
              <el-icon><User /></el-icon>
              <span>管理员</span>
            </div>
          </div>
        </el-header>
        
        <el-main class="main-content">
          <router-view />
        </el-main>
      </el-container>
    </el-container>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';

const route = useRoute();

const pageTitle = computed(() => {
  const titles: Record<string, string> = {
    '/dashboard': '仪表盘',
    '/strategy': '策略管理',
    '/backtest': '回测系统',
    '/trading': '交易执行',
    '/risk': '风险管理',
    '/monitor': '实时监控',
    '/settings': '系统设置',
    '/test': '测试页面',
  };
  return titles[route.path] || '量化交易系统';
});
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
