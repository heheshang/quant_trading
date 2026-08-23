//! Pipeline concrete implementations.
//!
//! This module provides [`make_risk_check_step`], [`make_order_exec_step`] and
//! [`make_order_processor_exec_step`] / [`make_live_pipeline`] helpers that wrap
//! real `risk_layer` / `trading_layer` / `OrderProcessor` engines into
//! strategy-layer pipeline steps via trait objects (DIP).

mod steps;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use strategy_engine::pipeline::{PipelineExecutor, RiskCheckStep};

use crate::order_processor::OrderProcessor;

pub use steps::{make_order_exec_step, make_order_processor_exec_step, make_risk_check_step};

/// Assemble a strategy-scheduler pipeline that routes orders through
/// [`OrderProcessor`].
///
/// The risk step is a passthrough because [`OrderProcessor::place_order`] runs
/// the authoritative pre-trade risk check with *live* account/position data
/// (fail-closed when enabled). Keeping this step passthrough avoids a redundant
/// (and potentially false-rejecting) double-check against fabricated zeroed
/// account state. The execution step delegates to `place_order`, which handles
/// paper / live routing, persistence, event emission and async execution.
pub fn make_live_pipeline(processor: Arc<OrderProcessor>) -> PipelineExecutor {
    let risk_step = RiskCheckStep::passthrough();
    let exec_step = make_order_processor_exec_step(processor);
    PipelineExecutor::with_steps(vec![Box::new(risk_step), Box::new(exec_step)])
}
