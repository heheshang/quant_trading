# Handler 层编码 Spec（Rust）

> HTTP Handler / axum 路由实现规范。

## 代码结构

```rust
// crates/handlers/src/price.rs

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use my_service_common::errors::AppError;
use my_service_domain::{ItemId, Money, PriceRule};
use my_service_services::PriceService;

/// 查询商品价格
#[utoipa::path(
    get,
    path = "/api/v1/items/{item_id}/price",
    responses(
        (status = 200, description = "价格查询成功", body = PriceResponse),
        (status = 404, description = "商品不存在"),
    )
)]
#[tracing::instrument(skip(state), fields(item_id = %item_id))]
pub async fn get_price(
    State(state): State<AppState>,
    Path(item_id): Path<i64>,
) -> Result<Json<PriceResponse>, AppError> {
    let item_id = ItemId(item_id);

    let price = state
        .price_service
        .get_price(item_id)
        .await
        .context("Failed to get price")?;

    match price {
        Some(p) => Ok(Json(PriceResponse::from(p))),
        None => Err(AppError::NotFound {
            resource: "item",
            id: item_id.0.to_string(),
        }),
    }
}

/// 价格查询响应
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PriceResponse {
    /// 商品 ID
    pub item_id: i64,
    /// 价格（单位：分）
    pub price: i64,
    /// 货币代码
    pub currency: String,
}

impl From<(ItemId, Money)> for PriceResponse {
    fn from((item_id, price): (ItemId, Money)) -> Self {
        Self {
            item_id: item_id.0,
            price: price.as_cents(),
            currency: "CNY".to_string(),
        }
    }
}
```

## 路由装配

```rust
// crates/app/src/router.rs

use axum::{routing::get, Router};
use my_service_handlers::price;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/items/{item_id}/price", get(price::get_price))
        .route("/api/v1/prices/batch", post(price::batch_get_prices))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::timeout::TimeoutLayer::new(
            std::time::Duration::from_secs(30),
        ))
        .with_state(state)
}
```

## 规范

### 1. Handler 函数签名

- 使用 `async fn`，接受 `State<AppState>` 作为第一个参数
- 使用 axum extractor 提取路径参数、查询参数、请求体
- 返回 `Result<impl IntoResponse, AppError>`
- 必须标注 `#[tracing::instrument]`，`skip(state)`

### 2. 参数校验

- 使用 `validator` crate 的 `#[validate]` derive 进行请求体校验
- 在 handler 中调用 `.validate()?` 或使用自定义 extractor
- 自定义 extractor 实现 `FromRequestParts` 或 `FromRequest`

```rust
/// 价格查询请求
#[derive(Debug, Deserialize, Validate)]
pub struct BatchPriceRequest {
    #[validate(length(min = 1, max = 100))]
    pub item_ids: Vec<i64>,
}

pub async fn batch_get_prices(
    State(state): State<AppState>,
    Json(req): Json<BatchPriceRequest>,
) -> Result<Json<Vec<PriceResponse>>, AppError> {
    req.validate()
        .context("Invalid request parameters")?;
    // ...
}
```

### 3. 异常处理

- Handler 不捕获错误 — 通过 `?` 传播到 axum 的全局错误处理
- `AppError` 必须实现 `IntoResponse`，转换为 HTTP 状态码 + JSON 错误体
- 使用 `tower` 中间件统一处理超时和限流

```rust
// crates/common/src/errors.rs

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {resource} id={id}")]
    NotFound { resource: &'static str, id: String },

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            AppError::NotFound { .. } => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

### 4. 统一响应格式

```rust
/// 统一成功响应
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }
}
```

### 5. OpenAPI 文档

- 使用 `utoipa` 标注 handler 和类型
- 每个 handler 必须有 `#[utoipa::path(...)]` 标注
- 请求/响应类型实现 `utoipa::ToSchema`
- 路由装配时注册 OpenAPI 路径

### 6. 日志规范

- 使用 `#[tracing::instrument]` 自动记录 handler 进入/退出
- INFO 级别：请求开始（含关键参数）
- WARN 级别：业务异常（如找不到资源）
- ERROR 级别：系统异常（如数据库连接失败）
- 禁止打印请求体/响应体中的敏感信息（密码、手机号）
- 结构化字段使用 `%` 或 `?` 格式化

### 7. 禁止的操作

- Handler 不写业务逻辑 — 全部委托给 Service
- Handler 不直接操作数据库 — 必须通过 Service → Repository
- Handler 不调用外部服务 — 必须通过 Service → Client
- Handler 不编排多个 Service 调用 — 编排逻辑在 Service 层
- Handler 不 panic — 所有错误通过 `AppError` 处理
