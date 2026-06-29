import { computed } from 'vue';
import { useStrategyStore } from '@/stores/strategy';
import type { StrategyParams, StrategyStatus } from '@/services/types';
import { useFormatting } from './useFormatting';

const { formatCurrency, formatPercentage, formatDate, formatStrategyType } = useFormatting();

export function useStrategyFormat() {
  const store = useStrategyStore();

  // Format functions (already available from useFormatting)
  const formatCurrencyFn = (value: number | string): string => formatCurrency(value);
  const formatPercentageFn = (value: number | string): string => formatPercentage(value);
  const formatDateFn = (dateInput: string | Date): string => formatDate(dateInput);
  const formatStrategyTypeFn = (type: string): string => formatStrategyType(type);

  // Status helper functions
  function getStatusTag(strategy: StrategyParams): 'active' | 'inactive' | 'pending' | 'error' | 'warning' | 'draft' {
    const status = strategy.status as StrategyStatus | undefined;
    if (!status || status === 'Draft') return 'draft';
    if (status === 'Running') return 'active';
    if (status === 'Paused') return 'warning';
    if (status === 'Archived') return 'inactive';
    if (status === 'Deployed') return 'pending';
    if (status === 'Backtesting') return 'pending';
    return 'draft';
  }

  function isRunningStatus(strategy: StrategyParams): boolean {
    const status = strategy.status as StrategyStatus | undefined;
    return status === 'Running';
  }

  // Computed for form dialog
  const strategyTypes = computed(() => store.strategyTypes);

  return {
    formatCurrency: formatCurrencyFn,
    formatPercentage: formatPercentageFn,
    formatDate: formatDateFn,
    formatStrategyType: formatStrategyTypeFn,
    getStatusTag,
    isRunningStatus,
    strategyTypes,
  };
}

export type UseStrategyFormatReturn = ReturnType<typeof useStrategyFormat>;