import { ref, computed, type ComputedRef, type Ref } from 'vue'

/**
 * Chart theme — reacts to the app's dark-mode toggle.
 *
 * `App.vue` toggles `dark` on `<html>`; a `MutationObserver` keeps `isDark`
 * in sync so charts rebuild their options with theme-aware colors when the
 * user switches light/dark.
 */

const isDark: Ref<boolean> = ref(document.documentElement.classList.contains('dark'))

let observer: MutationObserver | null = null
function ensureObserver() {
  if (observer || typeof MutationObserver === 'undefined') return
  observer = new MutationObserver(() => {
    isDark.value = document.documentElement.classList.contains('dark')
  })
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] })
}
ensureObserver()

/** Theme-aware ECharts color palette. */
export interface ChartPalette {
  text: string
  axisLabel: string
  splitLine: string
  tooltipBg: string
  tooltipText: string
  tooltipBorder: string
}

export function useChartTheme(): { isDark: Ref<boolean>; palette: ComputedRef<ChartPalette> } {
  const palette = computed<ChartPalette>(() => ({
    text: isDark.value ? '#e5eaf3' : '#303133',
    axisLabel: isDark.value ? '#a3a6ad' : '#606266',
    splitLine: isDark.value ? '#363637' : '#ebeef5',
    tooltipBg: isDark.value ? '#1d1e1f' : '#ffffff',
    tooltipText: isDark.value ? '#e5eaf3' : '#303133',
    tooltipBorder: isDark.value ? '#363637' : '#ebeef5',
  }))
  return { isDark, palette }
}
