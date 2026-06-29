import { computed, type ComputedRef, type Ref } from 'vue'
import { EditPen, DataAnalysis, Upload, VideoPlay, VideoPause, Box } from '@element-plus/icons-vue'
import type { StrategyStatus } from '@/services/types'

/**
 * Element Plus tag semantic type. Keep narrow — `danger` reserved for future error states.
 */
export type StrategyStatusType = 'info' | 'warning' | 'primary' | 'success' | 'danger'

/**
 * UI presentation for a {@link StrategyStatus} value. Single source of truth.
 */
export interface StrategyStatusDisplay {
  /** Chinese label rendered inside the tag. */
  readonly label: string
  /** Element Plus `<el-tag :type>` value. */
  readonly type: StrategyStatusType
  /** Element Plus icon component (Vue SFC / functional component). */
  readonly icon: ReturnType<typeof getIconComponent>
}

/**
 * Internal helper: Element Plus icons are Vue components (objects), not strings.
 * Wrapping in a function gives TypeScript a stable return type for the
 * `icon` field without losing precise typing.
 */
function getIconComponent(c: unknown): unknown {
  return c
}

const STATUS_MAP: Readonly<Record<StrategyStatus, StrategyStatusDisplay>> = {
  Draft:        { label: '草稿',   type: 'info',    icon: getIconComponent(EditPen) },
  Backtesting:  { label: '回测中', type: 'warning', icon: getIconComponent(DataAnalysis) },
  Deployed:     { label: '已部署', type: 'primary', icon: getIconComponent(Upload) },
  Running:      { label: '运行中', type: 'success', icon: getIconComponent(VideoPlay) },
  Paused:       { label: '已暂停', type: 'warning', icon: getIconComponent(VideoPause) },
  Archived:     { label: '已归档', type: 'info',    icon: getIconComponent(Box) },
}

/**
 * Resolve UI presentation for a strategy status.
 *
 * Accepts either a `Ref` or `ComputedRef<StrategyStatus>`; returns a
 * `ComputedRef<StrategyStatusDisplay>` so consumers can read `display.value`
 * once per render and destructure for templates / props.
 *
 * @example
 *   const status = computed(() => strategy.status)  // StrategyStatus
 *   const display = useStrategyStatus(status)
 *   <el-tag :type="display.value.type">{{ display.value.label }}</el-tag>
 */
export function useStrategyStatus(
  status: ComputedRef<StrategyStatus> | Ref<StrategyStatus>,
): ComputedRef<StrategyStatusDisplay> {
  return computed<StrategyStatusDisplay>(() => STATUS_MAP[status.value])
}
