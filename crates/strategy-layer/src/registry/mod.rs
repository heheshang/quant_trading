//! Strategy Registry — 策略注册中心
//!
//! 支持动态注册多种策略类型，消除硬编码策略选择的耦合。
//! 通过 `StrategyFactory` trait 与 `StrategyRegistry` 管理策略生命周期。

use async_trait::async_trait;
use quant_common::types::{ParameterSchema, StrategyParams};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::strategy::Strategy;

mod factories;
pub use factories::{MeanReversionFactory, TrendFollowingFactory};

// ─── FactoryError ─────────────────────────────────────────────────────────

/// 策略工厂与注册中心统一使用的错误类型。
///
/// 设计目标：
/// - 让 `create` 调用方能够精确区分「类型未注册」「参数非法」「初始化失败」
///   三类失败模式，而不是把它们都降级为字符串。
#[derive(Debug, Error)]
pub enum FactoryError {
    /// 注册中心中未找到指定策略类型
    #[error("Unknown strategy type '{0}'")]
    UnknownType(String),

    /// 参数未通过前置校验（例如缺字段、范围越界）
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// 策略 `initialize` 阶段失败
    #[error("Failed to initialize strategy: {0}")]
    Initialize(String),
}

// ─── StrategyFactory ──────────────────────────────────────────────────────

/// 策略工厂：根据参数创建策略实例 + 暴露参数 Schema
#[async_trait]
pub trait StrategyFactory: Send + Sync {
    /// 根据参数创建策略实例
    ///
    /// 实现必须在返回前完成 `initialize`，因此返回值已是「可运行的策略」。
    /// 任何前置校验失败、`initialize` 错误都必须以 `FactoryError` 形式传播，
    /// 不允许静默吞错后返回半初始化实例。
    async fn create(&self, params: StrategyParams) -> Result<Box<dyn Strategy>, FactoryError>;

    /// 返回该策略类型的参数 Schema（用于前端动态渲染参数配置界面）
    fn parameter_schema(&self) -> Vec<ParameterSchema>;
}

// ─── StrategyTypeInfo ─────────────────────────────────────────────────────

/// 已注册策略类型的元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyTypeInfo {
    /// 策略类型标识（如 "MeanReversion"）
    pub type_name: String,
    /// 可读名称
    pub display_name: String,
    /// 描述
    pub description: String,
    /// 参数 Schema
    pub parameters: Vec<ParameterSchema>,
}

// ─── StrategyRegistry ─────────────────────────────────────────────────────

/// 策略注册中心
///
/// 管理所有可用策略类型的工厂注册，支持动态注册/注销/查询。
/// 是整个系统的「策略类型目录」。
pub struct StrategyRegistry {
    factories: HashMap<String, Box<dyn StrategyFactory>>,
    display_names: HashMap<String, String>,
    descriptions: HashMap<String, String>,
}

impl StrategyRegistry {
    /// 创建空的注册中心
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
            display_names: HashMap::new(),
            descriptions: HashMap::new(),
        }
    }

    /// 注册一个策略类型
    ///
    /// - `type_name`：策略类型标识（如 `"MeanReversion"`）
    /// - `factory`：策略工厂实例
    /// - `display_name`：前端展示名称
    /// - `description`：策略描述
    pub fn register(
        &mut self,
        type_name: impl Into<String>,
        factory: Box<dyn StrategyFactory>,
        display_name: impl Into<String>,
        description: impl Into<String>,
    ) {
        let key = type_name.into();
        self.factories.insert(key.clone(), factory);
        self.display_names.insert(key.clone(), display_name.into());
        self.descriptions.insert(key, description.into());
    }

    /// 注销一个策略类型。返回 `true` 如果之前存在
    pub fn unregister(&mut self, type_name: &str) -> bool {
        let existed = self.factories.remove(type_name).is_some();
        self.display_names.remove(type_name);
        self.descriptions.remove(type_name);
        existed
    }

    /// 根据策略类型名称和参数创建策略实例
    ///
    /// 错误以 `FactoryError` 形式传播：
    /// - `FactoryError::UnknownType`：类型未注册
    /// - `FactoryError::Initialize` / `InvalidParameters`：工厂内部失败
    pub async fn create(
        &self,
        type_name: &str,
        params: StrategyParams,
    ) -> Result<Box<dyn Strategy>, FactoryError> {
        let factory = self
            .factories
            .get(type_name)
            .ok_or_else(|| FactoryError::UnknownType(type_name.to_string()))?;
        factory.create(params).await
    }

    /// 检查是否包含指定策略类型
    #[must_use]
    pub fn has_type(&self, type_name: &str) -> bool {
        self.factories.contains_key(type_name)
    }

    /// 获取单个策略类型的元数据
    #[must_use]
    pub fn get_type_info(&self, type_name: &str) -> Option<StrategyTypeInfo> {
        let factory = self.factories.get(type_name)?;
        Some(StrategyTypeInfo {
            type_name: type_name.to_string(),
            display_name: self
                .display_names
                .get(type_name)
                .cloned()
                .unwrap_or_default(),
            description: self
                .descriptions
                .get(type_name)
                .cloned()
                .unwrap_or_default(),
            parameters: factory.parameter_schema(),
        })
    }

    /// 列出所有已注册的策略类型名称
    #[must_use]
    pub fn list_type_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.factories.keys().cloned().collect();
        names.sort();
        names
    }

    /// 列出所有已注册策略类型的完整元数据
    #[must_use]
    pub fn list_types(&self) -> Vec<StrategyTypeInfo> {
        let mut types: Vec<StrategyTypeInfo> = self
            .factories
            .keys()
            .filter_map(|name| self.get_type_info(name))
            .collect();
        types.sort_by(|a, b| a.type_name.cmp(&b.type_name));
        types
    }

    /// 已注册的策略类型数量
    #[must_use]
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// 是否为空
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Helper: build default registry ───────────────────────────────────────

/// 创建预注册了内置策略类型的默认注册中心
#[must_use]
pub fn default_registry() -> StrategyRegistry {
    let mut registry = StrategyRegistry::new();
    registry.register(
        "MeanReversion",
        Box::new(MeanReversionFactory),
        "均值回归策略",
        "基于布林带和 RSI 的均值回归策略，在价格偏离均值时反向开仓",
    );
    registry.register(
        "TrendFollowing",
        Box::new(TrendFollowingFactory),
        "趋势跟踪策略",
        "基于 EMA 交叉的趋势跟踪策略，短期 EMA 上穿长期 EMA 时买入，下穿时卖出",
    );
    registry
}

#[cfg(test)]
mod tests;
