//! Strategy Registry — 策略注册中心
//!
//! 支持动态注册多种策略类型，消除硬编码策略选择的耦合。
//! 通过 `StrategyFactory` trait 与 `StrategyRegistry` 管理策略生命周期。

use async_trait::async_trait;
use quant_common::types::{ParameterSchema, StrategyParams};
use quant_common::Result;
use std::collections::HashMap;

use crate::strategy::{MeanReversionStrategy, Strategy};

// ─── StrategyFactory ──────────────────────────────────────────────────────

/// 策略工厂：根据参数创建策略实例 + 暴露参数 Schema
#[async_trait]
pub trait StrategyFactory: Send + Sync {
    /// 根据参数创建策略实例
    async fn create(&self, params: StrategyParams) -> Box<dyn Strategy>;

    /// 返回该策略类型的参数 Schema（用于前端动态渲染参数配置界面）
    fn parameter_schema(&self) -> Vec<ParameterSchema>;
}

// ─── StrategyTypeInfo ─────────────────────────────────────────────────────

/// 已注册策略类型的元数据
#[derive(Debug, Clone)]
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
    pub async fn create(
        &self,
        type_name: &str,
        params: StrategyParams,
    ) -> Result<Box<dyn Strategy>> {
        let factory = self.factories.get(type_name).ok_or_else(|| {
            quant_common::Error::Internal(format!(
                "Unknown strategy type '{}'. Available: {:?}",
                type_name,
                self.list_type_names(),
            ))
        })?;
        Ok(factory.create(params).await)
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

// ─── MeanReversionFactory ─────────────────────────────────────────────────

/// 均值回归策略的工厂实现
pub struct MeanReversionFactory;

#[async_trait]
impl StrategyFactory for MeanReversionFactory {
    async fn create(&self, params: StrategyParams) -> Box<dyn Strategy> {
        let mut strategy = MeanReversionStrategy::new();
        // 忽略 initialize 的错误——工厂只负责创建，
        // 参数验证由调用方或 initialize 自身完成
        let _ = strategy.initialize(params).await;
        Box::new(strategy)
    }

    fn parameter_schema(&self) -> Vec<ParameterSchema> {
        vec![
            ParameterSchema {
                name: "lookback_period".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(20),
                range: Some(quant_common::types::ParamRange {
                    min: 5.0,
                    max: 100.0,
                    step: Some(1.0),
                }),
                description: "Lookback period for mean reversion calculation".into(),
            },
            ParameterSchema {
                name: "entry_threshold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(2.0),
                range: Some(quant_common::types::ParamRange {
                    min: 0.5,
                    max: 5.0,
                    step: Some(0.1),
                }),
                description: "Entry threshold in standard deviations".into(),
            },
            ParameterSchema {
                name: "exit_threshold".into(),
                param_type: quant_common::types::ParamType::Number,
                default: serde_json::json!(0.5),
                range: Some(quant_common::types::ParamRange {
                    min: 0.1,
                    max: 3.0,
                    step: Some(0.1),
                }),
                description: "Exit threshold in standard deviations".into(),
            },
        ]
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
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_empty_by_default() {
        let registry = StrategyRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.list_type_names().is_empty());
    }

    #[tokio::test]
    async fn test_register_and_query() {
        let mut registry = StrategyRegistry::new();
        registry.register(
            "MeanReversion",
            Box::new(MeanReversionFactory),
            "均值回归",
            "基于 RSI 的均值回归",
        );

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.has_type("MeanReversion"));
        assert!(!registry.has_type("TrendFollowing"));

        let names = registry.list_type_names();
        assert_eq!(names, vec!["MeanReversion"]);
    }

    #[tokio::test]
    async fn test_create_via_registry() {
        let mut registry = StrategyRegistry::new();
        registry.register(
            "MeanReversion",
            Box::new(MeanReversionFactory),
            "均值回归",
            "",
        );

        let params = StrategyParams {
            strategy_id: "test_001".to_string(),
            strategy_name: "Test MR".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({
                "lookback_period": 20,
                "entry_threshold": 2.0,
                "exit_threshold": 0.5,
            }),
            enabled: true,
            max_position: rust_decimal::Decimal::new(100000, 0),
            max_daily_loss: rust_decimal::Decimal::new(5000, 0),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            status: quant_common::types::StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
        };

        let strategy = registry.create("MeanReversion", params).await;
        assert!(strategy.is_ok());
        assert_eq!(strategy.unwrap().name(), "Test MR");
    }

    #[tokio::test]
    async fn test_create_unknown_type_returns_error() {
        let registry = StrategyRegistry::new();
        let params = StrategyParams {
            strategy_id: "t".to_string(),
            strategy_name: "T".to_string(),
            strategy_type: quant_common::types::StrategyType::MeanReversion,
            params: serde_json::json!({}),
            enabled: true,
            max_position: rust_decimal::Decimal::ZERO,
            max_daily_loss: rust_decimal::Decimal::ZERO,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            status: quant_common::types::StrategyStatus::Draft,
            description: None,
            tags: vec![],
            symbols: vec![],
        };
        let result = registry.create("NonExistent", params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unregister() {
        let mut registry = StrategyRegistry::new();
        registry.register(
            "MeanReversion",
            Box::new(MeanReversionFactory),
            "",
            "",
        );
        assert!(registry.has_type("MeanReversion"));

        let removed = registry.unregister("MeanReversion");
        assert!(removed);
        assert!(!registry.has_type("MeanReversion"));
        assert!(registry.is_empty());
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_returns_false() {
        let mut registry = StrategyRegistry::new();
        assert!(!registry.unregister("NonExistent"));
    }

    #[tokio::test]
    async fn test_get_type_info() {
        let mut registry = StrategyRegistry::new();
        registry.register(
            "MeanReversion",
            Box::new(MeanReversionFactory),
            "均值回归策略",
            "基于 RSI 的均值回归",
        );

        let info = registry.get_type_info("MeanReversion");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.type_name, "MeanReversion");
        assert_eq!(info.display_name, "均值回归策略");
        assert_eq!(info.description, "基于 RSI 的均值回归");
        assert!(!info.parameters.is_empty());
    }

    #[tokio::test]
    async fn test_list_types() {
        let mut registry = StrategyRegistry::new();
        registry.register(
            "MeanReversion",
            Box::new(MeanReversionFactory),
            "均值回归",
            "MR strategy",
        );

        let types = registry.list_types();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].type_name, "MeanReversion");
    }

    #[tokio::test]
    async fn test_default_registry_has_mean_reversion() {
        let registry = default_registry();
        assert_eq!(registry.len(), 1);
        assert!(registry.has_type("MeanReversion"));
    }

    #[tokio::test]
    async fn test_mean_reversion_factory_schema() {
        let factory = MeanReversionFactory;
        let schema = factory.parameter_schema();
        assert_eq!(schema.len(), 3);

        let names: Vec<&str> = schema.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"lookback_period"));
        assert!(names.contains(&"entry_threshold"));
        assert!(names.contains(&"exit_threshold"));
    }
}
