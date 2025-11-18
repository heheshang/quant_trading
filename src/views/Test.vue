<template>
  <div>
    <h2>测试页面</h2>
    <p>测试功能开发中...</p>
    
    <el-card style="margin-top: 20px;">
      <template #header>
        <div class="card-header">
          <span>系统功能导航</span>
        </div>
      </template>
      
      <div style="display: flex; flex-wrap: wrap; gap: 10px;">
        <el-button @click="goTo('/dashboard')" type="primary">仪表盘</el-button>
        <el-button @click="goTo('/strategy')" type="primary">策略管理</el-button>
        <el-button @click="goTo('/backtest')" type="primary">回测系统</el-button>
        <el-button @click="goTo('/trading')" type="primary">交易执行</el-button>
        <el-button @click="goTo('/risk')" type="primary">风险管理</el-button>
        <el-button @click="goTo('/monitor')" type="primary">实时监控</el-button>
        <el-button @click="goTo('/settings')" type="primary">系统设置</el-button>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router';

const router = useRouter();

// Navigate to a route
const goTo = (path: string) => {
  router.push(path);
};
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>