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

/**
 * Resolve the `--chart-*` CSS variables to concrete JS color strings.
 *
 * ECharts renders to a `<canvas>`, so `var(--x)` strings are unusable there.
 * This reads the actual computed CSS custom property values at call time and
 * falls back to the token defaults when the stylesheet is unavailable (e.g.
 * in jsdom unit tests). Names map directly to the `--chart-*` token keys.
 */
export function getChartSeriesColors(
  names: string[] = ['blue', 'green', 'red', 'orange', 'purple', 'teal', 'gray'],
): Record<string, string> {
  const fallback: Record<string, string> = {
    blue: '#409eff',
    green: '#67c23a',
    red: '#f56c6c',
    orange: '#e6a23c',
    purple: '#9b59b6',
    teal: '#1abc9c',
    gray: '#95a5a6',
  }
  let styles: CSSStyleDeclaration | null = null
  if (typeof document !== 'undefined' && document.documentElement) {
    styles = window.getComputedStyle(document.documentElement)
  }
  const out: Record<string, string> = {}
  for (const name of names) {
    const raw = styles?.getPropertyValue(`--chart-${name}`).trim()
    out[name] = raw || fallback[name]
  }
  return out
}
