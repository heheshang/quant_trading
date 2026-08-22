<template>
  <div>
    <el-card class="okx-section-card">
      <template #header>
        <div class="card-header">
          <span>交易对列表</span>
          <el-button size="small" @click="$emit('refreshInstruments')">刷新</el-button>
        </div>
      </template>
      <el-table :data="instruments.slice(0, showAll ? instruments.length : 10)" size="small" style="width: 100%" max-height="200" v-loading="instrumentsLoading">
        <el-table-column prop="instId" label="产品ID" width="120" />
        <el-table-column prop="baseCcy" label="基础币" width="80" />
        <el-table-column prop="quoteCcy" label="计价币" width="80" />
        <el-table-column prop="instType" label="类型" width="80" />
      </el-table>
      <div v-if="instruments.length > 10" class="show-more" @click="showAll = !showAll">
        {{ showAll ? '收起' : '显示全部 (' + instruments.length + ')' }}
      </div>
    </el-card>
    <el-card class="okx-section-card" style="margin-top: 12px;">
      <template #header>
        <div class="card-header">
          <span>OKX 公告</span>
          <el-button size="small" :loading="announcementsLoading" @click="$emit('refreshAnnouncements')">刷新</el-button>
        </div>
      </template>
      <div v-if="announcements.length === 0" class="market-data-placeholder">暂无公告</div>
      <ul v-else class="announcement-list">
        <li v-for="(item, idx) in announcements.slice(0, 5)" :key="idx">
          <a :href="item.url" target="_blank" rel="noopener"># {{ item.title || item.notice }}</a>
        </li>
      </ul>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

defineProps<{
  instruments: any[]
  instrumentsLoading: boolean
  announcements: any[]
  announcementsLoading: boolean
}>()

defineEmits<{
  refreshInstruments: []
  refreshAnnouncements: []
}>()

const showAll = ref(false)

defineExpose({ showAll })
</script>

<style scoped>
.okx-section-card {
  margin-bottom: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.show-more {
  text-align: center;
  padding: 8px;
  color: #409EFF;
  cursor: pointer;
  font-size: 13px;
}
.show-more:hover {
  text-decoration: underline;
}
.market-data-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  color: var(--color-text-secondary);
}
.announcement-list {
  list-style: none;
  margin: 0;
  padding: 4px 12px;
}
.announcement-list li {
  padding: 4px 0;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.announcement-list a {
  color: var(--color-text-regular);
  text-decoration: none;
}
.announcement-list a:hover {
  color: #409EFF;
  text-decoration: underline;
}
</style>
