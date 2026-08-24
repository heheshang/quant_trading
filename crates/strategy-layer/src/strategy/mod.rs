//! 策略接口与具体策略实现。
//!
//! 每个具体策略一个子文件；本模块承担：
//!   1. `Strategy` trait + `StrategyContext` 定义
//!   2. 子模块声明 + re-export（保持外部 `crate::strategy::*` 访问语义）

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quant_common::types::{MarketData, Order, ParameterSchema, Position, StrategyParams};
use quant_common::Result;

pub mod mean_reversion;
pub mod trend_following;
pub mod macd;
pub mod rsi;

pub use mean_reversion::MeanReversionStrategy;
pub use trend_following::TrendFollowingStrategy;
pub use macd::MacdStrategy;
pub use rsi::RsiStrategy;

/// 策略上下文
pub struct StrategyContext {
    pub current_time: DateTime<Utc>,
    pub positions: Vec<Position>,
    pub market_data: Vec<MarketData>,
}

/// 策略接口
#[async_trait]
pub trait Strategy: Send + Sync {
    /// 初始化策略
    async fn initialize(&mut self, params: StrategyParams) -> Result<()>;

    /// 生成交易信号
    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>>;

    /// 策略名称
    fn name(&self) -> &str;

    /// 策略参数
    fn params(&self) -> &StrategyParams;

    /// 策略参数的可变访问器。`update_params` 的默认实现依赖此方法。
    fn params_mut(&mut self) -> &mut StrategyParams;

    /// 更新策略参数（仅元数据，不重置运行时状态）
    ///
    /// 默认实现只替换 `self.params`，不会重新解析已派生的运行时阈值。
    /// 若策略需要更复杂的热更新语义，可重写此方法。
    async fn update_params(&mut self, params: StrategyParams) -> Result<()> {
        *self.params_mut() = params;
        Ok(())
    }

    /// 显式重新初始化：完整重置策略状态并应用新参数。
    ///
    /// 默认实现直接转发给 `initialize`，调用方需自行承担重置带来的状态丢失。
    async fn reinitialize(&mut self, params: StrategyParams) -> Result<()> {
        self.initialize(params).await
    }

    // ── 生命周期钩子（默认空实现） ─────────────────────────────────────

    /// 策略部署时调用
    async fn on_deploy(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略启动时调用
    async fn on_start(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略停止时调用
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略暂停时调用
    async fn on_pause(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略恢复时调用
    async fn on_resume(&mut self) -> Result<()> {
        Ok(())
    }

    /// 策略归档时调用
    async fn on_archive(&mut self) -> Result<()> {
        Ok(())
    }

    // ── 参数 Schema ─────────────────────────────────────────────────────

    /// 返回该策略的参数 Schema 定义（用于前端动态渲染和参数校验）
    #[must_use]
    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        Vec::new()
    }
}
