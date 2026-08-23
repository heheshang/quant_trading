use crate::state::AppState;
use quant_services::{expand_grid, OptimizationAlgorithm, OptimizationMetric, OptimizationResult};
use rust_decimal::prelude::FromPrimitive;
use serde_json::json;
use tauri::State;

/// 参数优化命令：用 GridSearch 在参数网格上跑回测，并按指标打分。
///
/// - `param_grid`：对象（`{"rsi_period": [14, 7]}` → 笛卡尔积）或已展开的数组。
/// - `metric`：`sharpe_ratio` / `annual_return` / `max_drawdown`。
/// - `algorithm`：仅 `grid_search`（默认）；`bayesian` / `genetic` 返回 NotImplemented。
///
/// 返回 top-N 组合与最优参数。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn optimize_strategy(
    state: State<'_, AppState>,
    strategy_id: String,
    param_grid: serde_json::Value,
    metric: String,
    algorithm: Option<String>,
    top_n: Option<usize>,
    initial_capital: Option<f64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<serde_json::Value, String> {
    let services = state
        .app_services
        .as_ref()
        .ok_or("Application services not initialized")?;

    let metric = OptimizationMetric::parse(&metric).map_err(|e| e.to_string())?;
    let algorithm =
        OptimizationAlgorithm::parse(&algorithm.unwrap_or_else(|| "grid_search".to_string()))
            .map_err(|e| e.to_string())?;
    let top_n = top_n.unwrap_or(5);

    let grid = expand_grid(&param_grid).map_err(|e| e.to_string())?;
    if grid.is_empty() {
        return Err("Parameter grid is empty".to_string());
    }

    let start = parse_date(start_date.as_deref(), "start", 180)?;
    let end = parse_date(end_date.as_deref(), "end", 0)?;

    let (strategy_type, market_data) = services
        .strategy_service
        .prepare_optimization_input(&strategy_id, start, end)
        .await
        .map_err(|e| e.to_string())?;

    if market_data.is_empty() {
        return Err(format!(
            "No market data returned for strategy '{strategy_id}' in the requested date range"
        ));
    }

    let init_cap = rust_decimal::Decimal::from_f64(initial_capital.unwrap_or(10_000.0))
        .ok_or_else(|| "Invalid initial capital".to_string())?;

    let result = services
        .optimizer
        .optimize_with_algorithm(
            algorithm,
            &strategy_type,
            grid,
            market_data,
            init_cap,
            rust_decimal::Decimal::ZERO,
            rust_decimal::Decimal::ZERO,
            metric,
        )
        .await
        .map_err(|e| e.to_string())?;

    let OptimizationResult {
        total_combinations,
        combinations,
        best,
        ..
    } = result;
    let returned = combinations.len().min(top_n);
    let top = combinations.into_iter().take(top_n).collect::<Vec<_>>();

    Ok(json!({
        "total_combinations": total_combinations,
        "combinations_returned": returned,
        "top_n_requested": top_n,
        "combinations": top,
        "best": best,
    }))
}

/// 解析可选的日期字符串；缺省时相对今天偏移 `days_ago` 天。
fn parse_date(
    s: Option<&str>,
    label: &str,
    days_ago: i64,
) -> Result<chrono::DateTime<chrono::Utc>, String> {
    match s {
        Some(ds) => chrono::DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", ds))
            .map(|d| d.with_timezone(&chrono::Utc))
            .map_err(|e| format!("Invalid {label} date: {}", e)),
        None => Ok(chrono::Utc::now() - chrono::Duration::days(days_ago)),
    }
}
