<template>
  <div class="test-page">
    <h2>测试页面</h2>
    <p style="color:var(--color-text-regular);margin-bottom:20px;">系统功能测试与状态验证</p>

    <el-row :gutter="20">
      <el-col :span="8">
        <el-card>
          <template #header><div class="card-header"><span>API 服务</span></div></template>
          <div style="text-align:center;padding:20px 0;">
            <el-icon :size="36" :color="apiStatus.api ? 'var(--color-success)' : 'var(--color-danger)'"><Connection /></el-icon>
            <p style="margin-top:8px;font-size:14px;">{{ apiStatus.api ? '连接正常' : '连接失败' }}</p>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card>
          <template #header><div class="card-header"><span>数据库</span></div></template>
          <div style="text-align:center;padding:20px 0;">
            <el-icon :size="36" :color="apiStatus.db ? 'var(--color-success)' : 'var(--color-danger)'"><Connection /></el-icon>
            <p style="margin-top:8px;font-size:14px;">{{ apiStatus.db ? '连接正常' : '连接失败' }}</p>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card>
          <template #header><div class="card-header"><span>Redis</span></div></template>
          <div style="text-align:center;padding:20px 0;">
            <el-icon :size="36" :color="apiStatus.redis ? 'var(--color-success)' : 'var(--color-danger)'"><Connection /></el-icon>
            <p style="margin-top:8px;font-size:14px;">{{ apiStatus.redis ? '连接正常' : '连接失败' }}</p>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" class="equal-row" style="margin-top:20px;">
      <el-col :span="12">
        <el-card>
          <template #header><div class="card-header"><span>运行测试</span></div></template>
          <div style="padding:12px 0;">
            <el-button type="primary" @click="runSystemTests" :loading="testing" style="width:100%">
              {{ testing ? '测试中...' : '运行系统测试' }}
            </el-button>
          </div>
          <el-table v-if="testResults.length > 0" :data="testResults" style="width:100%">
            <el-table-column prop="name" label="测试项" />
            <el-table-column prop="status" label="结果" width="80">
              <template #default="scope">
                <el-tag :type="scope.row.status === '通过' ? 'success' : 'danger'" size="small">
                  {{ scope.row.status }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="detail" label="详情" min-width="180" show-overflow-tooltip />
            <el-table-column prop="duration" label="耗时" width="80" />
          </el-table>
          <EmptyState v-else-if="!testing" title="尚未运行测试" description="点击上方按钮执行系统测试" />
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card>
          <template #header><div class="card-header"><span>系统信息</span></div></template>
          <el-descriptions :column="1" border>
            <el-descriptions-item label="系统版本">v1.0.0</el-descriptions-item>
            <el-descriptions-item label="Tauri 版本">2.0</el-descriptions-item>
            <el-descriptions-item label="Vue 版本">3.4</el-descriptions-item>
            <el-descriptions-item label="Element Plus">最新</el-descriptions-item>
            <el-descriptions-item label="数据库状态">{{ apiStatus.db ? '已连接' : '未连接' }}</el-descriptions-item>
            <el-descriptions-item label="Redis 状态">{{ apiStatus.redis ? '已连接' : '未连接' }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { Connection } from '@element-plus/icons-vue';
import { checkRedisStatus, getMetrics } from '@/services/monitor'
import { getAccountInfo } from '@/services/account'
import { verifyToken } from '@/services/auth';
import EmptyState from '@/components/common/EmptyState.vue';

const apiStatus = reactive({
  api: false,
  db: false,
  redis: false,
});

const testing = ref(false);
const testResults = ref<Array<{name: string; status: string; duration: string; detail?: string}>>([]);

// Check API connectivity on mount
async function checkConnectivity() {
  try {
    await getMetrics();
    apiStatus.api = true;
  } catch {
    apiStatus.api = false;
  }
  try {
    apiStatus.redis = await checkRedisStatus();
  } catch {
    apiStatus.redis = false;
  }
}
void checkConnectivity();

async function runSystemTests() {
  testing.value = true;
  testResults.value = [];
  try {
    const tests = [
      { name: 'API 接口测试', fn: testAPI },
      { name: '数据库连接', fn: testDatabase },
      { name: 'Redis 缓存', fn: testRedis },
      { name: 'JWT 认证', fn: testAuth },
    ];
    for (const t of tests) {
      const start = performance.now();
      const { passed, detail } = await t.fn();
      const duration = ((performance.now() - start) / 1000).toFixed(2) + 's';
      testResults.value.push({ name: t.name, status: passed ? '通过' : '失败', duration, detail });
    }
  } finally {
    testing.value = false;
  }
}

async function testAPI(): Promise<{passed: boolean; detail?: string}> {
  try {
    await getMetrics();
    apiStatus.api = true;
    return { passed: true, detail: 'getMetrics() 成功' };
  } catch (e: any) {
    apiStatus.api = false;
    return { passed: false, detail: e?.message || '请求失败' };
  }
}

async function testDatabase(): Promise<{passed: boolean; detail?: string}> {
  try {
    const result = await getAccountInfo();
    apiStatus.db = true;
    return { passed: true, detail: `account_id=${result.account_id}` };
  } catch (e: any) {
    apiStatus.db = false;
    return { passed: false, detail: e?.message || '数据库连接失败' };
  }
}

async function testRedis(): Promise<{passed: boolean; detail?: string}> {
  try {
    const healthy = await checkRedisStatus();
    apiStatus.redis = healthy;
    return {
      passed: healthy,
      detail: healthy ? 'Redis PING 返回 PONG' : 'Redis PING 未返回 PONG',
    };
  } catch (e: any) {
    apiStatus.redis = false;
    return { passed: false, detail: e?.message || 'Redis 连接失败' };
  }
}

async function testAuth(): Promise<{passed: boolean; detail?: string}> {
  const token = localStorage.getItem('authToken');
  if (!token) return { passed: false, detail: '未找到 token' };
  try {
    const valid = await verifyToken(token);
    return { passed: valid, detail: valid ? 'Token 有效' : 'Token 无效' };
  } catch (e: any) {
    return { passed: false, detail: e?.message || '验证请求失败' };
  }
}
</script>

<style scoped>
.test-page {
  padding: 20px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.equal-row .el-col {
  display: flex;
}
.equal-row .el-card {
  flex: 1;
  width: 100%;
}
</style>
