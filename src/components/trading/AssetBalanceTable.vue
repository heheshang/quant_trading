<template>
  <el-card class="asset-balance-card">
    <template #header>
      <div class="card-header">
        <span>{{ title }}（总市值 ¥{{ formatCurrency(totalValue) }}）</span>
        <div class="controls">
          <el-select
            v-model="assetFilter"
            filterable
            clearable
            placeholder="资产下拉搜索"
            style="width: 180px"
          >
            <el-option v-for="a in assetOptions" :key="a" :label="a" :value="a" />
          </el-select>
          <el-button size="small" @click="emit('refresh')">刷新</el-button>
        </div>
      </div>
    </template>

    <el-table :data="paginated" size="small" v-loading="loading">
      <el-table-column prop="asset" label="资产" />
      <el-table-column prop="free" label="可用" />
      <el-table-column prop="locked" label="锁定" />
    </el-table>

    <EmptyState v-if="!loading && filtered.length === 0" title="暂无余额" />

    <Paginator
      v-if="filtered.length > 0"
      :total="filtered.length"
      :page="page"
      :page-size="pageSize"
      @update:page="page = $event"
      @update:pageSize="pageSize = $event; page = 1"
    />
  </el-card>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import Paginator from '@/components/common/Paginator.vue'
import EmptyState from '@/components/common/EmptyState.vue'
import { useFormatting } from '@/composables/useFormatting'
import { getLastPrices } from '@/services/market'
import type { BinanceBalance } from '@/services/types'

/**
 * 共享「账户余额」卡：总市值 + 可筛选下拉 + 分页。
 * Trading 实盘余额卡与 Binance 交易面板余额卡复用。
 */
const props = withDefaults(
  defineProps<{
    balances: BinanceBalance[]
    title?: string
    loading?: boolean
  }>(),
  { title: '账户余额', loading: false },
)
const emit = defineEmits<{ refresh: [] }>()

const { formatCurrency } = useFormatting()
const assetFilter = ref('')
const page = ref(1)
const pageSize = ref(10)
const prices = ref<Record<string, number>>({})

const USD_STABLES = ['USDT', 'USDC', 'TUSD', 'BUSD', 'FDUSD', 'DAI']
const priceOf = (asset: string) =>
  USD_STABLES.includes(asset) ? 1 : prices.value[asset + 'USDT'] || 0

const totalValue = computed(() =>
  props.balances.reduce(
    (s, b) => s + ((Number(b.free) || 0) + (Number(b.locked) || 0)) * priceOf(b.asset),
    0,
  ),
)
const assetOptions = computed(() =>
  props.balances
    .filter((b) => Number(b.free) > 0 || Number(b.locked) > 0)
    .map((b) => b.asset),
)
const filtered = computed(() =>
  assetFilter.value
    ? props.balances.filter((b) => b.asset === assetFilter.value)
    : props.balances.filter((b) => Number(b.free) > 0 || Number(b.locked) > 0),
)
const paginated = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filtered.value.slice(start, start + pageSize.value)
})

watch(assetFilter, () => { page.value = 1 })

async function fetchPrices() {
  try {
    // 从 DB 读全标的最近价（快照写入器 60s 落库），前端不再直连币安 REST。
    const rows = await getLastPrices()
    const m: Record<string, number> = {}
    for (const r of rows) m[r.symbol] = r.price
    prices.value = m
  } catch {
    // 市场价格拉取失败不影响余额/总市值（按 0 计， degrade）
  }
}
onMounted(fetchPrices)
watch(() => props.balances, fetchPrices)
</script>

<style scoped>
.asset-balance-card { margin-bottom: 20px; }
.card-header { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
.controls { display: flex; align-items: center; gap: 8px; }
</style>
