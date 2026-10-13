//! 后台任务模块
//!
//! 职责：
//! - 纸面订单执行调度器启动
//! - Prometheus 指标服务器
//! - Binance WebSocket 后台任务

use crate::commands::binance_ws;
use quant_services::AppServices;
use tauri::AppHandle;
use tracing::info;

/// 启动所有后台任务
pub fn start_background_tasks(app_handle: AppHandle, app_services: &AppServices) {
    // 启动实盘订单状态监控
    binance_ws::start_live_order_monitor(app_handle.clone(), app_services);
    info!("Binance live order monitor started");

    // 启动资产曲线后台快照写入器
    binance_ws::start_equity_snapshot_writer(app_services);
    info!("Equity snapshot writer started");

    // 启动监控指标刷新器
    binance_ws::start_monitor_metrics(app_services);
    info!("Monitor metrics updater started");
}
