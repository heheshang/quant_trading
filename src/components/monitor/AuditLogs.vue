<template>
  <div class="audit-logs">
    <div class="audit-toolbar">
      <el-input
        v-model="filterUsername"
        placeholder="按用户名过滤"
        clearable
        style="width: 200px"
        @change="onFilterChange"
      />
      <el-input
        v-model="filterAction"
        placeholder="按动作过滤（如 Login / OrderSubmit）"
        clearable
        style="width: 260px"
        @change="onFilterChange"
      />
      <el-button @click="fetchLogs">刷新</el-button>
    </div>

    <el-table :data="logs" v-loading="loading" style="width: 100%">
      <el-table-column prop="timestamp" label="时间" width="190" />
      <el-table-column label="用户ID" width="110">
        <template #default="{ row }">{{ row.user_id || '—' }}</template>
      </el-table-column>
      <el-table-column prop="username" label="用户名" width="130" />
      <el-table-column prop="action" label="动作" width="150">
        <template #default="scope">
          <el-tag size="small">{{ scope.row.action }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="resource" label="资源" min-width="160" show-overflow-tooltip />
      <el-table-column prop="ip_address" label="IP" width="130">
        <template #default="scope">{{ scope.row.ip_address ?? '—' }}</template>
      </el-table-column>
      <el-table-column label="结果" width="90">
        <template #default="scope">
          <el-tag :type="scope.row.success ? 'success' : 'danger'" size="small">
            {{ scope.row.success ? '成功' : '失败' }}
          </el-tag>
        </template>
      </el-table-column>
    </el-table>

    <div class="audit-footer">
      <el-pagination
        v-model:current-page="currentPage"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="prev, pager, next, sizes"
        @current-change="fetchLogs"
        @size-change="onPageSizeChange"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getAuditLogs, type AuditLog } from '@/services/audit'

const logs = ref<AuditLog[]>([])
const loading = ref(false)
const filterAction = ref('')
const filterUsername = ref('')
const currentPage = ref(1)
const pageSize = ref(20)
const total = ref(0)

function onFilterChange() {
  currentPage.value = 1
  fetchLogs()
}

function onPageSizeChange(size: number) {
  pageSize.value = size
  currentPage.value = 1
  fetchLogs()
}

async function fetchLogs() {
  loading.value = true
  try {
    const offset = (currentPage.value - 1) * pageSize.value
    const data = await getAuditLogs({
      username: filterUsername.value || undefined,
      action: filterAction.value || undefined,
      limit: pageSize.value,
      offset,
    })
    logs.value = Array.isArray(data) ? data : []
    // The backend does not return a total count; infer one from the page size.
    total.value =
      data.length < pageSize.value
        ? offset + data.length
        : offset + data.length + 1
  } catch (error) {
    console.error('Failed to fetch audit logs:', error)
    logs.value = []
  } finally {
    loading.value = false
  }
}

onMounted(fetchLogs)
</script>

<style scoped>
.audit-logs {
  padding: 4px 0;
}
.audit-toolbar {
  display: flex;
  gap: 12px;
  margin-bottom: 12px;
}
.audit-footer {
  margin-top: 12px;
  display: flex;
  justify-content: flex-end;
}
</style>
