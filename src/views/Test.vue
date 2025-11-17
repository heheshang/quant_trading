<template>
  <div class="test">
    <h1>Test Page</h1>
    <el-button @click="testCommand">Test Tauri Command</el-button>
    <p>{{ result }}</p>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';

const result = ref('');

async function testCommand() {
  try {
    // @ts-ignore
    const data = await window.__TAURI__.invoke('get_account_info');
    result.value = JSON.stringify(data, null, 2);
    console.log('Test result:', data);
  } catch (error) {
    console.error('Test failed:', error);
    result.value = 'Error: ' + error;
  }
}
</script>

<style scoped>
.test {
  padding: 20px;
}
</style>
