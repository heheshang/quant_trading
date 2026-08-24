import { invoke } from '@tauri-apps/api/core'

/**
 * 统一 API 错误（携带业务码 + 后端真实错误信息）。
 */
export class ApiError extends Error {
  readonly code: number
  constructor(code: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.code = code
  }
}

/** 统一响应信封（与后端 `quant_common::api::ApiResponse<T>` 对应）。 */
interface ApiEnvelope<T> {
  code: number
  message: string
  data: T | null
}

function toApiError(e: unknown): ApiError {
  // 后端 `Err(ApiFailure)`（Tauri 序列化为 `{code,message}`）或字符串 JSON。
  if (e !== null && typeof e === 'object' && 'code' in e && 'message' in e) {
    const f = e as { code: number; message: string }
    return new ApiError(f.code, f.message)
  }
  if (typeof e === 'string') {
    try {
      const p = JSON.parse(e)
      if (p && 'code' in p && 'message' in p) return new ApiError(p.code, p.message)
    } catch {
      // not JSON
    }
    return new ApiError(5001, e)
  }
  return new ApiError(5001, (e as Error)?.message || '未知错误')
}

/**
 * Single IPC transport (DIP).
 *
 * 已迁移 command 返回 `Result<ApiResponse<T>, ApiFailure>`：
 * - 成功解包返回 `data`；
 * - 失败（Err(ApiFailure) 或信封 code!==0）抛 `ApiError(code, message)`（前端展示真实错误）。
 * 未迁移 command 返回裸数据，直接返回（向后兼容）。
 */
export function call<T>(cmd: string, args?: Args): Promise<T> {
  const p = args === undefined ? invoke<unknown>(cmd) : invoke<unknown>(cmd, args)
  return p.then(
    (res) => {
      if (res !== null && typeof res === 'object' && 'code' in res && 'message' in res) {
        const env = res as unknown as ApiEnvelope<T>
        if (env.code !== 0) {
          throw new ApiError(env.code, env.message)
        }
        return env.data as T
      }
      return res as T
    },
    (e: unknown) => {
      throw toApiError(e)
    },
  )
}

type Args = Record<string, unknown>
