//! StrategyRegistry 单元测试。

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

    let params = StrategyParams::builder(
        "test_001".to_string(),
        "Test MR".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({
        "lookback_period": 20,
        "entry_threshold": 2.0,
        "exit_threshold": 0.5,
    }))
    .max_position(rust_decimal::Decimal::new(100000, 0))
    .max_daily_loss(rust_decimal::Decimal::new(5000, 0))
    .build();

    let strategy = registry.create("MeanReversion", params).await;
    assert!(strategy.is_ok());
    assert_eq!(strategy.unwrap().name(), "Test MR");
}

#[tokio::test]
async fn test_create_unknown_type_returns_error() {
    let registry = StrategyRegistry::new();
    let params = StrategyParams::builder(
        "t".to_string(),
        "T".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(rust_decimal::Decimal::ZERO)
    .max_daily_loss(rust_decimal::Decimal::ZERO)
    .build();
    let result = registry.create("NonExistent", params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unregister() {
    let mut registry = StrategyRegistry::new();
    registry.register("MeanReversion", Box::new(MeanReversionFactory), "", "");
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
async fn test_default_registry_has_builtin_strategies() {
    let registry = default_registry();
    assert_eq!(registry.len(), 2);
    assert!(registry.has_type("MeanReversion"));
    assert!(registry.has_type("TrendFollowing"));
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

/// P0-2: `StrategyFactory::create` must return a `Result` so that
/// `initialize` failures are propagated instead of silently swallowed.
#[tokio::test]
async fn test_factory_create_propagates_initialize_errors() {
    struct FailingFactory;

    #[async_trait]
    impl StrategyFactory for FailingFactory {
        async fn create(
            &self,
            _params: StrategyParams,
        ) -> Result<Box<dyn crate::strategy::Strategy>, FactoryError> {
            Err(FactoryError::Initialize("forced failure".to_string()))
        }

        fn parameter_schema(&self) -> Vec<ParameterSchema> {
            Vec::new()
        }
    }

    let factory = FailingFactory;
    let params = StrategyParams::builder(
        "fail-001".to_string(),
        "Failing".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(rust_decimal::Decimal::ZERO)
    .max_daily_loss(rust_decimal::Decimal::ZERO)
    .build();

    let result = factory.create(params).await;
    assert!(
        result.is_err(),
        "factory.create must return Err on init failure"
    );
    match result.err().unwrap() {
        FactoryError::Initialize(msg) => assert_eq!(msg, "forced failure"),
        other => panic!("expected FactoryError::Initialize, got {other:?}"),
    }
}

/// P0-2: `StrategyRegistry::create` must surface a typed `FactoryError`
/// when the requested type is unknown — preserving the registry's
/// "type catalog" semantics for callers.
#[tokio::test]
async fn test_registry_create_unknown_type_returns_typed_error() {
    let registry = StrategyRegistry::new();
    let params = StrategyParams::builder(
        "x".to_string(),
        "X".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(rust_decimal::Decimal::ZERO)
    .max_daily_loss(rust_decimal::Decimal::ZERO)
    .build();

    let result = registry.create("NonExistent", params).await;
    assert!(result.is_err());
    match result.err().unwrap() {
        FactoryError::UnknownType(name) => assert_eq!(name, "NonExistent"),
        other => panic!("expected FactoryError::UnknownType, got {other:?}"),
    }
}

/// P0-2: a factory that propagates a typed `FactoryError::InvalidParameters`
/// must reach the caller intact (not be downgraded to a `String`).
#[tokio::test]
async fn test_registry_propagates_invalid_parameters_error() {
    struct BadParamFactory;

    #[async_trait]
    impl StrategyFactory for BadParamFactory {
        async fn create(
            &self,
            _params: StrategyParams,
        ) -> Result<Box<dyn crate::strategy::Strategy>, FactoryError> {
            Err(FactoryError::InvalidParameters(
                "missing lookback_period".to_string(),
            ))
        }

        fn parameter_schema(&self) -> Vec<ParameterSchema> {
            Vec::new()
        }
    }

    let mut registry = StrategyRegistry::new();
    registry.register(
        "BadParam",
        Box::new(BadParamFactory),
        "BadParam",
        "always fails parameter validation",
    );

    let params = StrategyParams::builder(
        "bp-1".to_string(),
        "BP".to_string(),
        quant_common::types::StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(rust_decimal::Decimal::ZERO)
    .max_daily_loss(rust_decimal::Decimal::ZERO)
    .build();

    let result = registry.create("BadParam", params).await;
    match result.err().unwrap() {
        FactoryError::InvalidParameters(msg) => {
            assert!(msg.contains("lookback_period"));
        }
        other => panic!("expected FactoryError::InvalidParameters, got {other:?}"),
    }
}
