//! 领域类型。
//!
//! 按业务域拆分为 `market` / `strategy` 两个子模块，此处 re-export 保持 `types::*` 访问语义。

pub mod market;
pub mod strategy;

pub use market::*;
pub use strategy::*;

#[cfg(test)]
mod tests;
