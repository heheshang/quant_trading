<template>
  <div class="chart-controls" v-if="showControls">
    <el-radio-group :model-value="timeRange" @update:model-value="onTimeRange" size="small">
      <el-radio-button label="1D">1D</el-radio-button>
      <el-radio-button label="1W">1W</el-radio-button>
      <el-radio-button label="1M">1M</el-radio-button>
      <el-radio-button label="3M">3M</el-radio-button>
      <el-radio-button label="1Y">1Y</el-radio-button>
    </el-radio-group>

    <el-select :model-value="dataSource" @update:model-value="onDataSource" size="small" style="width: 120px">
      <el-option label="回测数据" value="backtest" />
      <el-option label="实时数据" value="realtime" />
    </el-select>
  </div>
</template>

<script setup lang="ts">
/**
 * Chart controls — time range + data source selectors.
 *
 * Renders an Element Plus radio group (1D/1W/1M/3M/1Y) and a select
 * (backtest/realtime). Emits a single `change` event when either control
 * changes; the parent owns the v-model state via `timeRange` and `dataSource`
 * props and decides what to refetch.
 */
const props = withDefaults(
  defineProps<{
    timeRange: string
    dataSource: 'backtest' | 'realtime'
    showControls?: boolean
  }>(),
  { showControls: true },
)
void props // suppress unused warning while keeping the prop public API for clarity

const emit = defineEmits<{
  'update:timeRange': [value: string]
  'update:dataSource': [value: 'backtest' | 'realtime']
  change: []
}>()

function onTimeRange(value: string) {
  emit('update:timeRange', value)
  emit('change')
}

function onDataSource(value: 'backtest' | 'realtime') {
  emit('update:dataSource', value)
  emit('change')
}
</script>

<style scoped>
.chart-controls {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 10;
  display: flex;
  gap: 8px;
  align-items: center;
  background: rgba(255, 255, 255, 0.9);
  padding: 4px 8px;
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}
</style>
