# API 契约（前后端统一响应结构）

> 所有 Tauri `command` 统一返回 `Result<ApiResponse<T>, ApiFailure>`：
> - 成功：`{ code: 0, message: "ok", data: <payload> }`
> - 失败：`Err(ApiFailure { code, message })`（Tauri 序列化为 `{ code, message }` 供前端捕获）
>
> 前端 `services/transport.ts` 的 `call<T>` 负责解包：成功返回 `data`；失败抛
> `ApiError(code, message)`。**操作失败展示后端真实错误信息**。

## 错误码（`quant_common::api::code`）

| 码 | 常量 | 含义 |
|---|---|---|
| 0 | `OK` | 成功 |
| 1001 | `UNAUTHORIZED` | 未登录 / 会话失效 |
| 1003 | `FORBIDDEN` | 无权限 / 角色不足 |
| 2001 | `INVALID_PARAM` | 参数无效 |
| 2002 | `VALIDATION_FAILED` | 校验失败 |
| 2101 | `RATE_LIMITED` | 限流 |
| 3001 | `RISK_REJECTED` | 风控拒绝 |
| 4001 | `NOT_FOUND` | 资源不存在 |
| 4002 | `CONFLICT` | 冲突（并发修改等） |
| 5001 | `INTERNAL` | 内部错误 |
| 5002 | `DATABASE` | 数据库错误 |
| 5003 | `BINANCE_API` | Binance API 错误 |
| 5004 | `DATA_SOURCE` | 数据源错误 |
| 5005 | `STRATEGY` | 策略错误 |
| 5006 | `BACKTEST` | 回测错误 |
| 5007 | `NOT_INITIALIZED` | 服务未初始化 |

## 响应样例

```jsonc
// 成功
{ "code": 0, "message": "ok", "data": { "order_id": 123 } }
// 失败（Err(ApiFailure)）
{ "code": 3001, "message": "Insufficient position to sell. Symbol: BTC-USDT, Available: 0, Required: 0.1" }
```

## 服务层映射

`ServiceError` 已实现 `api_code()` / `api_message()` / `to_api_result()`：
```rust
// 命令内：
service_account.get_account_info().await.map_err(|e| e.to_api_result::<Account>())?;
// 或直接透传：
return service_error.to_api_result();
```

## 前端用法

```ts
import { call, ApiError } from '@/services/transport'

async function submit() {
  try {
    const orderId = await call<string>('submit_order', { order })
  } catch (e) {
    if (e instanceof ApiError) ElMessage.error(`${e.code} ${e.message}`) // 展示后端真实错误
  }
}
```

## 迁移状态

- **已迁移**（返回信封）：`submit_order` / `cancel_order` / `get_account_info` / `get_positions` / `get_recent_orders`。
- **未迁移**（返回裸数据，`call` 向后兼容直通）：其余命令。迁移同模式：改返回类型 + 用 `ok_result`/`err_result`/`to_api_result`。
- **前端 transport** 对两者自适应（裸数据直通；信封解包 + 抛 `ApiError`）。
