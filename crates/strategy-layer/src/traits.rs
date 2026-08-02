//! Strategy layer abstractions (Dependency Inversion Principle)
//!
//! These traits decouple the strategy-layer from concrete implementations
//! in risk-layer, trading-layer, and data-layer. Downstream crates
//! (e.g., services) provide the implementations.

use crate::signals::Signal;
use async_trait::async_trait;

/// Risk check abstraction.
///
/// Implementations wrap concrete risk checkers (e.g., `PreTradeRiskChecker`)
/// and are injected into pipeline steps.
#[async_trait]
pub trait RiskChecker: Send + Sync {
    /// Check whether a signal passes risk constraints.
    ///
    /// # Errors
    ///
    /// Returns `RiskCheckError::Rejected` when the signal is blocked.
    /// Returns `RiskCheckError::Internal` for unexpected failures.
    async fn check(&self, signal: &Signal) -> Result<(), RiskCheckError>;
}

/// Order execution abstraction.
///
/// Implementations wrap concrete execution engines (e.g., `ExecutionEngine`)
/// and are injected into pipeline steps.
#[async_trait]
pub trait OrderExecutor: Send + Sync {
    /// Execute a signal as an order.
    ///
    /// # Returns
    ///
    /// An order identifier on success.
    ///
    /// # Errors
    ///
    /// Returns `OrderExecError::Rejected` when execution is rejected.
    /// Returns `OrderExecError::Internal` for unexpected failures.
    async fn execute(&self, signal: &Signal) -> Result<String, OrderExecError>;
}

/// Risk check error types.
#[derive(thiserror::Error, Debug)]
pub enum RiskCheckError {
    #[error("Risk check failed: {0}")]
    Rejected(String),

    #[error("Risk check internal error: {0}")]
    Internal(String),
}

/// Order execution error types.
#[derive(thiserror::Error, Debug)]
pub enum OrderExecError {
    #[error("Order execution rejected: {0}")]
    Rejected(String),

    #[error("Order execution internal error: {0}")]
    Internal(String),
}
