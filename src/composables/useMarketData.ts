import { storeToRefs } from 'pinia'
import { useMarketDataStore } from '@/stores/marketData'

/**
 * Realtime Binance market data composable.
 *
 * Thin wrapper over the {@link useMarketDataStore} store so components can
 * consume `$store`-style state without importing the store directly.
 */
export function useMarketData() {
  const store = useMarketDataStore()
  const refs = storeToRefs(store)
  return {
    ...refs,
    start: store.start,
    stop: store.stop,
    setActiveSymbol: store.setActiveSymbol,
  }
}
