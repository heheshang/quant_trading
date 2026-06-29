//! Pipeline concrete implementations.
//!
//! This module provides [`make_risk_check_step`] and [`make_order_exec_step`]
//! helpers that wrap real `risk_layer` / `trading_layer` engines into
//! strategy-layer pipeline steps via trait objects (DIP).

mod steps;

#[cfg(test)]
mod tests;

pub use steps::{make_order_exec_step, make_risk_check_step};
