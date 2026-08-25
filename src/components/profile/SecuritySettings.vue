<template>
  <el-card class="security-card">
    <template #header>
      <div class="card-header">
        <span>安全设置</span>
      </div>
    </template>

    <div class="security-settings">
      <el-button type="primary" style="width: 100%; margin-bottom: 10px;" @click="$emit('changePassword')">
        修改密码
      </el-button>
      <el-button style="width: 100%;" @click="$emit('setup2fa')">
        双因素认证
      </el-button>
    </div>
  </el-card>
</template>

<script setup lang="ts">
defineEmits<{
  changePassword: []
  setup2fa: []
}>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}


.security-settings {
  padding: 20px 0;
}
</style>
