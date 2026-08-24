//! 统一 API 响应结构（前后端契约）。
//!
//! 所有 Tauri command 返回 `ApiResponse<T>`：
//! - 成功：`{ code: 0, message: "ok", data: <payload> }`
//! - 失败：`{ code: <非0错误码>, message: "<真实错误信息>", data: null }`
//!
//! 前端 `services/transport.ts` 的 `call<T>` 负责解包：`code===0` 返回 `data`，
//! 否则抛出带 `code`+`message` 的 `ApiError`，操作失败时展示后端真实错误信息。

use serde::{Deserialize, Serialize};

/// 统一响应信封。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

/// 业务错误码（分段：1xxx 认证 / 2xxx 参数校验 / 3xxx 风控 / 4xxx 资源 / 5xxx 服务）。
pub mod code {
    pub const OK: i32 = 0;

    // 1xxx 认证/权限
    /// 未登录 / 会话失效
    pub const UNAUTHORIZED: i32 = 1001;
    /// 无权限 / 角色不足
    pub const FORBIDDEN: i32 = 1003;

    // 2xxx 参数/校验
    pub const INVALID_PARAM: i32 = 2001;
    pub const VALIDATION_FAILED: i32 = 2002;

    // 2.1 限流
    pub const RATE_LIMITED: i32 = 2101;

    // 3xxx 风控
    pub const RISK_REJECTED: i32 = 3001;

    // 4xxx 资源/冲突
    pub const NOT_FOUND: i32 = 4001;
    pub const CONFLICT: i32 = 4002;

    // 5xxx 服务/外部
    pub const INTERNAL: i32 = 5001;
    pub const DATABASE: i32 = 5002;
    pub const BINANCE_API: i32 = 5003;
    pub const DATA_SOURCE: i32 = 5004;
    pub const STRATEGY: i32 = 5005;
    pub const BACKTEST: i32 = 5006;
    pub const NOT_INITIALIZED: i32 = 5007;
}

impl<T> ApiResponse<T> {
    /// 成功响应。
    pub fn ok(data: T) -> Self {
        Self { code: code::OK, message: "ok".to_string(), data: Some(data) }
    }

    /// 失败响应（携带业务码 + 真实错误信息）。
    pub fn err(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), data: None }
    }
}

/// 便捷空成功（命令无返回值时）。
pub type ApiEmpty = ApiResponse<serde_json::Value>;

/// `() -> ApiEmpty`。
pub fn ok_empty() -> ApiEmpty {
    ApiResponse { code: code::OK, message: "ok".to_string(), data: Some(serde_json::Value::Null) }
}

/// 命令失败载荷（Tauri `Result<_, ApiFailure>` 的 Err 序列化为 `{code,message}`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiFailure {
    pub code: i32,
    pub message: String,
}

impl ApiFailure {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

/// 便捷：`Result<T, ApiFailure>` 别名。
pub type ApiResult<T> = Result<T, ApiFailure>;

/// 成功命令结果。
pub fn ok_result<T>(data: T) -> ApiResult<ApiResponse<T>> {
    Ok(ApiResponse::ok(data))
}

/// 失败命令结果。
pub fn err_result<T>(code: i32, message: impl Into<String>) -> ApiResult<ApiResponse<T>> {
    Err(ApiFailure::new(code, message))
}
