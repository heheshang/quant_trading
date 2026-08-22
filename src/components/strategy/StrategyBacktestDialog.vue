<template>
  <el-dialog v-model="localVisible" title="回测结果" width="800px">
    <div v-if="result">
      <el-row :gutter="20">
        <el-col :span="8">
          <el-card class="backtest-stat-card">
            <div class="stat-item">
              <div class="stat-label">总收益率</div>
              <div class="stat-value" :class="{ positive: result.total_return > 0, negative: result.total_return < 0 }">
                {{ formatPercentage(result.total_return) }}
              </div>
            </div>
          </el-card>
        </el-col>
        <el-col :span="8">
          <el-card class="backtest-stat-card">
            <div class="stat-item">
              <div class="stat-label">夏普比率</div>
              <div class="stat-value">{{ result.sharpe_ratio.toFixed(2) }}</div>
            </div>
          </el-card>
        </el-col>
        <el-col :span="8">
          <el-card class="backtest-stat-card">
            <div class="stat-item">
              <div class="stat-label">最大回撤</div>
              <div class="stat-value negative">{{ formatPercentage(result.max_drawdown) }}</div>
            </div>
          </el-card>
        </el-col>
      </el-row>

      <el-row :gutter="20" style="margin-top: 20px;">
        <el-col :span="24">
          <PerformanceChart
            :equity-curve="equityCurveData"
            :show-controls="false"
            height="300px"
          />
        </el-col>
      </el-row>
    </div>

    <template #footer>
      <span class="dialog-footer">
        <el-button @click="localVisible = false">关闭</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import PerformanceChart from './PerformanceChart.vue';
import { useFormatting } from '@/composables/useFormatting';

const { formatPercentage } = useFormatting();

const props = withDefaults(defineProps<{
  visible: boolean;
  result: any;
}>(), {
  result: null,
});

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const localVisible = computed({
  get: () => props.visible,
  set: (v) => emit('update:visible', v),
});

const equityCurveData = computed(() =>
  props.result?.equity_curve?.map((pair: [string, number]) => pair[1]) || []
);
</script>

<style scoped>
.backtest-stat-card { margin-bottom: 20px; }
.stat-item { text-align: center; padding: 8px 0; }
.stat-label { font-size: 14px; color: var(--color-text-secondary); margin-bottom: 8px; }
.stat-value { font-size: 24px; font-weight: bold; color: var(--color-text-primary); }
.stat-value.positive { color: #67c23a; }
.stat-value.negative { color: #f56c6c; }
</style>
