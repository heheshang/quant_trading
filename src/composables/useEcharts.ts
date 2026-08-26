import * as echarts from 'echarts/core'
import { ref, watch, onBeforeUnmount, onMounted, type Ref } from 'vue'

/**
 * ECharts lifecycle composable.
 *
 * Encapsulates the four phases of managing a chart instance:
 *   1. `init` — create the chart on the referenced DOM element
 *   2. `setOption` — react to `options` changes (deep watch)
 *   3. `resize` — sync the chart with the container size
 *   4. `dispose` — release resources on unmount
 *
 * The composable owns the chart instance and is the only place that
 * imports ECharts runtime APIs. Components using this composable don't
 * need to know how ECharts is initialized or torn down.
 *
 * @param elRef      - template ref pointing to the chart container `<div>`
 * @param options    - reactive chart options (deep-watched)
 * @returns reactive state + imperative lifecycle methods
 */
export function useEcharts(
  elRef: Ref<HTMLElement | null | undefined>,
  options: Ref<echarts.EChartsCoreOption>,
) {
  const instance = ref<echarts.ECharts | null>(null)
  const isReady = ref(false)
  let resizeObserver: ResizeObserver | null = null

  function init() {
    if (!elRef.value) return
    instance.value = echarts.init(elRef.value, undefined, {
      renderer: 'canvas',
      useDirtyRect: false,
    })
    instance.value.setOption(options.value)
    isReady.value = true
    observeResize()
  }

  function observeResize() {
    if (!elRef.value || typeof ResizeObserver === 'undefined') return
    resizeObserver?.disconnect()
    resizeObserver = new ResizeObserver(() => instance.value?.resize())
    resizeObserver.observe(elRef.value)
  }

  function resize() {
    instance.value?.resize()
  }

  function dispose() {
    resizeObserver?.disconnect()
    resizeObserver = null
    if (instance.value) {
      instance.value.dispose()
      instance.value = null
      isReady.value = false
    }
  }

  // Keep the chart in sync with options
  watch(
    options,
    (newOpts) => {
      instance.value?.setOption(newOpts, true)
    },
    { deep: true },
  )

  onMounted(() => {
    // Defer init to next tick so the parent has rendered the container
    queueMicrotask(init)
  })

  onBeforeUnmount(dispose)

  return { instance, isReady, init, resize, dispose }
}
