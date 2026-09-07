//! StrategyService 单元测试。

use super::*;
use async_trait::async_trait;
use data_layer::{RepoError, StrategyStats, StrategySummaryRow};
use mockall::mock;
use quant_common::config::SchedulerConfig;
use rust_decimal::prelude::FromPrimitive;

fn make_service_no_db() -> StrategyService {
    StrategyService::new(None, None, None, None, None)
}

#[tokio::test]
async fn get_strategies_no_db_returns_error() {
    let svc = make_service_no_db();
    let result = svc.get_strategies().await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[tokio::test]
async fn save_strategy_no_db_returns_error() {
    let svc = make_service_no_db();
    let strategy = StrategyParams::builder(
        "test_001".to_string(),
        "Test".to_string(),
        StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(Decimal::ZERO)
    .max_daily_loss(Decimal::ZERO)
    .build();
    let result = svc.save_strategy(&strategy).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[tokio::test]
async fn delete_strategy_no_db_returns_error() {
    let svc = make_service_no_db();
    let result = svc.delete_strategy("test_001").await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[tokio::test]
async fn toggle_strategy_no_db_returns_error() {
    let svc = make_service_no_db();
    let result = svc.toggle_strategy("test_001", true).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[tokio::test]
async fn run_backtest_no_db_returns_error() {
    let svc = make_service_no_db();
    let result = svc
        .run_backtest(
            "test_001",
            chrono::Utc::now() - chrono::Duration::days(7),
            chrono::Utc::now(),
            Decimal::from(100000),
            Decimal::from_f64(0.001).unwrap(),
            Decimal::from_f64(0.0005).unwrap(),
            &["BTC-USDT".to_string()],
            "1H",
        )
        .await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[test]
fn resolve_backtest_symbol_prefers_requested_symbols() {
    let mut params = mock_strategy_params(StrategyStatus::Draft);
    params.params = serde_json::json!({ "symbol": "ETH-USDT" });

    assert_eq!(
        StrategyService::resolve_backtest_symbol(&["BTC-USDT".to_string()], &params),
        "BTC-USDT"
    );
    assert_eq!(
        StrategyService::resolve_backtest_symbol(&["".to_string()], &params),
        "ETH-USDT"
    );
    assert_eq!(
        StrategyService::resolve_backtest_symbol(&[], &params),
        "ETH-USDT"
    );
    params.params = serde_json::json!({});
    assert_eq!(
        StrategyService::resolve_backtest_symbol(&[], &params),
        "BTC-USDT"
    );
}

#[tokio::test]
async fn get_backtest_results_no_db_returns_error() {
    let svc = make_service_no_db();
    let result = svc.get_backtest_results(20, 0).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[tokio::test]
async fn get_backtest_result_no_db_returns_error() {
    let svc = make_service_no_db();
    let result = svc.get_backtest_result(1).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[tokio::test]
async fn delete_backtest_result_no_db_returns_error() {
    let svc = make_service_no_db();
    let result = svc.delete_backtest_result(1).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ServiceError::DatabaseNotConnected
    ));
}

#[test]
fn new_with_none_creates_service_with_no_deps() {
    let svc = make_service_no_db();
    assert!(svc.postgres.is_none());
    assert!(svc.market_data_provider.is_none());
    assert!(svc.backtest_repo.is_none());
}

// ── Registry ─────────────────────────────────────────────────────────

#[tokio::test]
async fn list_strategy_types_no_registry_returns_error() {
    let svc = make_service_no_db();
    let result = svc.list_strategy_types();
    assert!(result.is_err());
}

#[tokio::test]
async fn list_strategy_types_with_registry() {
    let mut svc = make_service_no_db();
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let result = svc.list_strategy_types().unwrap();
    assert!(!result.is_empty());
    let names: Vec<&str> = result.iter().map(|t| t.type_name.as_str()).collect();
    for expected in ["MeanReversion", "TrendFollowing", "MACD", "RSI"] {
        assert!(names.contains(&expected), "missing type: {expected}");
    }
}

#[tokio::test]
async fn lifecycle_methods_no_db_returns_error() {
    let svc = make_service_no_db();
    assert!(svc.deploy_strategy("test_001").await.is_err());
    assert!(svc.start_strategy("test_001").await.is_err());
    assert!(svc.stop_strategy("test_001").await.is_err());
    assert!(svc.pause_strategy("test_001").await.is_err());
    assert!(svc.resume_strategy("test_001").await.is_err());
    assert!(svc.archive_strategy("test_001").await.is_err());
}

// ── Mock Strategy Repository ──────────────────────────────────────────────

mock! {
    pub StrategyRepo {}

    #[async_trait]
    impl StRepo for StrategyRepo {
        #[mockall::concretize]
        async fn find_all(
            &self,
            search: Option<&str>,
            strategy_type: Option<StrategyType>,
            status: Option<StrategyStatus>,
            enabled: Option<bool>,
            limit: i64,
            offset: i64,
        ) -> Result<(Vec<StrategySummaryRow>, i64), RepoError>;

        #[mockall::concretize]
        async fn count(
            &self,
            search: Option<&str>,
            strategy_type: Option<StrategyType>,
            status: Option<StrategyStatus>,
            enabled: Option<bool>,
        ) -> Result<i64, RepoError>;

        #[mockall::concretize]
        async fn find_by_id(&self, strategy_id: &str) -> Result<Option<StrategyParams>, RepoError>;

        async fn insert(&self, params: &StrategyParams) -> Result<i32, RepoError>;

        async fn update(&self, params: &StrategyParams) -> Result<bool, RepoError>;

        #[mockall::concretize]
        async fn update_with_version(
            &self,
            strategy_id: &str,
            params: &StrategyParams,
            expected_version: i64,
        ) -> Result<bool, RepoError>;

        #[mockall::concretize]
        async fn delete_by_id(&self, strategy_id: &str) -> Result<bool, RepoError>;

        #[mockall::concretize]
        async fn update_status(
            &self,
            strategy_id: &str,
            status: StrategyStatus,
            updated_by: Option<&str>,
        ) -> Result<bool, RepoError>;

        #[mockall::concretize]
        async fn update_status_if(
            &self,
            strategy_id: &str,
            new_status: StrategyStatus,
            expected_old_status: StrategyStatus,
            updated_by: Option<&str>,
        ) -> Result<bool, RepoError>;

        async fn stats(&self) -> Result<StrategyStats, RepoError>;
    }
}

// ── Convenience Helpers ───────────────────────────────────────────────────

fn mock_strategy_params(status: StrategyStatus) -> StrategyParams {
    StrategyParams::builder(
        "test_001".to_string(),
        "Test Strategy".to_string(),
        StrategyType::MeanReversion,
    )
    .params(serde_json::json!({}))
    .max_position(Decimal::ZERO)
    .max_daily_loss(Decimal::ZERO)
    .status(status)
    .build()
}

fn mock_summary_row(id: i32, strategy_id: &str) -> StrategySummaryRow {
    StrategySummaryRow {
        id,
        strategy_id: strategy_id.to_string(),
        strategy_name: "Test Strategy".to_string(),
        strategy_type: "MeanReversion".to_string(),
        params: serde_json::json!({}),
        enabled: true,
        status: "Draft".to_string(),
        max_position: Decimal::ZERO,
        max_daily_loss: Decimal::ZERO,
        description: None,
        instance_label: None,
        tags: serde_json::json!([]),
        symbols: serde_json::json!([]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        user_id: Some(0),
        version: 0,
    }
}

fn make_mock_service(repo: MockStrategyRepo, with_scheduler: bool) -> StrategyService {
    let strategy_repo: Arc<dyn StRepo> = Arc::new(repo);
    let scheduler = if with_scheduler {
        let sched_config = SchedulerConfig {
            enabled: true,
            ..Default::default()
        };
        let sched = StrategyScheduler::new(sched_config);
        // Configure a (passthrough) pipeline + mock provider so the scheduler
        // is "trading ready" and start_strategy can actually start the task.
        sched.set_pipeline(Arc::new(strategy_engine::PipelineExecutor::new()));
        sched.set_market_data_provider(Arc::new(
            crate::market_data_provider::MockMarketDataProvider::new(Vec::new()),
        ));
        Some(Arc::new(sched))
    } else {
        None
    };
    StrategyService::new(None, None, None, Some(strategy_repo), scheduler)
}

// ── deploy_strategy ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_deploy_strategy_from_backtesting() {
    let mut mock_repo = MockStrategyRepo::new();

    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Backtesting))));
    mock_repo
        .expect_update_status_if()
        .returning(|_, _, _, _| Ok(true));

    let svc = make_mock_service(mock_repo, false);
    let result = svc.deploy_strategy("test_001").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StrategyStatus::Deployed);
}

// ── start_strategy ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_start_strategy_from_deployed() {
    let mut mock_repo = MockStrategyRepo::new();

    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Deployed))));
    mock_repo
        .expect_update_status_if()
        .returning(|_, _, _, _| Ok(true));

    let svc = make_mock_service(mock_repo, true);
    let result = svc.start_strategy("test_001").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StrategyStatus::Running);
}

// ── stop_strategy ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_stop_strategy_from_running() {
    let mut mock_repo = MockStrategyRepo::new();

    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
    mock_repo
        .expect_update_status_if()
        .returning(|_, _, _, _| Ok(true));

    let svc = make_mock_service(mock_repo, true);
    let result = svc.stop_strategy("test_001").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StrategyStatus::Archived);
}

// ── pause_strategy ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_pause_strategy_from_running() {
    let mut mock_repo = MockStrategyRepo::new();

    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
    mock_repo
        .expect_update_status_if()
        .returning(|_, _, _, _| Ok(true));

    let svc = make_mock_service(mock_repo, true);
    let result = svc.pause_strategy("test_001").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StrategyStatus::Paused);
}

// ── resume_strategy ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_resume_strategy_from_paused() {
    let mut mock_repo = MockStrategyRepo::new();

    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Paused))));
    mock_repo
        .expect_update_status_if()
        .returning(|_, _, _, _| Ok(true));

    let svc = make_mock_service(mock_repo, true);
    let result = svc.resume_strategy("test_001").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StrategyStatus::Running);
}

// ── archive_strategy ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_archive_strategy_from_running() {
    let mut mock_repo = MockStrategyRepo::new();

    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
    mock_repo
        .expect_update_status_if()
        .returning(|_, _, _, _| Ok(true));

    let svc = make_mock_service(mock_repo, false);
    let result = svc.archive_strategy("test_001").await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), StrategyStatus::Archived);
}

// ── Illegal Status Transitions ────────────────────────────────────────────

#[tokio::test]
async fn test_deploy_strategy_from_running_rejected() {
    let mut mock_repo = MockStrategyRepo::new();
    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Running))));
    let svc = make_mock_service(mock_repo, false);
    let result = svc.deploy_strategy("test_001").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::InvalidStatusTransition { from, to } => {
            assert_eq!(from, "Running");
            assert_eq!(to, "Deployed");
        }
        other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_start_strategy_from_draft_rejected() {
    let mut mock_repo = MockStrategyRepo::new();
    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Draft))));
    let svc = make_mock_service(mock_repo, true);
    let result = svc.start_strategy("test_001").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::InvalidStatusTransition { from, to } => {
            assert_eq!(from, "Draft");
            assert_eq!(to, "Running");
        }
        other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_pause_strategy_from_deployed_rejected() {
    let mut mock_repo = MockStrategyRepo::new();
    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Deployed))));
    let svc = make_mock_service(mock_repo, true);
    let result = svc.pause_strategy("test_001").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::InvalidStatusTransition { from, to } => {
            assert_eq!(from, "Deployed");
            assert_eq!(to, "Paused");
        }
        other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_resume_strategy_from_draft_rejected() {
    let mut mock_repo = MockStrategyRepo::new();
    mock_repo
        .expect_find_by_id()
        .withf(|s: &str| s == "test_001")
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Draft))));
    let svc = make_mock_service(mock_repo, true);
    let result = svc.resume_strategy("test_001").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::InvalidStatusTransition { from, to } => {
            assert_eq!(from, "Draft");
            assert_eq!(to, "Running");
        }
        other => panic!("Expected InvalidStatusTransition, got: {:?}", other),
    }
}

// ── list_strategies pagination ────────────────────────────────────────────

#[tokio::test]
async fn test_list_strategies_invalid_page_zero() {
    let mock_repo = MockStrategyRepo::new();
    let svc = make_mock_service(mock_repo, false);

    let result = svc.list_strategies(None, 0, 20).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::PaginationInvalid { .. } => {}
        other => panic!("Expected PaginationInvalid, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_list_strategies_invalid_page_size_too_large() {
    let mock_repo = MockStrategyRepo::new();
    let svc = make_mock_service(mock_repo, false);

    let result = svc.list_strategies(None, 1, 101).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::PaginationInvalid { .. } => {}
        other => panic!("Expected PaginationInvalid, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_list_strategies_valid_pagination() {
    let mut mock_repo = MockStrategyRepo::new();

    let rows = vec![
        mock_summary_row(1, "strat_001"),
        mock_summary_row(2, "strat_002"),
    ];

    mock_repo
        .expect_find_all()
        .returning(move |_, _, _, _, _, _| Ok((rows.clone(), 2)));

    let svc = make_mock_service(mock_repo, false);
    let result = svc.list_strategies(None, 1, 20).await;

    assert!(result.is_ok());
    let strategies = result.unwrap();
    assert_eq!(strategies.len(), 2);
}

#[tokio::test]
async fn test_list_strategies_page_size_one() {
    let mut mock_repo = MockStrategyRepo::new();

    let rows = vec![mock_summary_row(1, "strat_001")];

    mock_repo
        .expect_find_all()
        .returning(move |_, _, _, _, _, _| Ok((rows.clone(), 1)));

    let svc = make_mock_service(mock_repo, false);
    let result = svc.list_strategies(None, 1, 1).await;

    assert!(result.is_ok());
    let strategies = result.unwrap();
    assert_eq!(strategies.len(), 1);
}

#[tokio::test]
async fn test_list_strategies_page_size_max() {
    let mut mock_repo = MockStrategyRepo::new();

    let rows = (1..=3)
        .map(|i| mock_summary_row(i, &format!("strat_{:03}", i)))
        .collect::<Vec<_>>();

    mock_repo
        .expect_find_all()
        .returning(move |_, _, _, _, _, _| Ok((rows.clone(), 3)));

    let svc = make_mock_service(mock_repo, false);
    let result = svc.list_strategies(None, 1, 100).await;

    assert!(result.is_ok());
    let strategies = result.unwrap();
    assert_eq!(strategies.len(), 3);
}

// ── save_strategy ─────────────────────────────────────────────────────────

struct SavingStrategyRepo {
    state: parking_lot::Mutex<Option<StrategyParams>>,
    reads: Option<tokio::sync::Barrier>,
}

impl SavingStrategyRepo {
    fn new(strategy: Option<StrategyParams>, concurrent: bool) -> Self {
        Self {
            state: parking_lot::Mutex::new(strategy),
            reads: concurrent.then(|| tokio::sync::Barrier::new(2)),
        }
    }
}

#[async_trait]
impl StRepo for SavingStrategyRepo {
    async fn find_by_id(&self, _: &str) -> Result<Option<StrategyParams>, RepoError> {
        let snapshot = self.state.lock().clone();
        if let Some(reads) = &self.reads {
            // Both requests must observe the same version (or absence) before either writes.
            reads.wait().await;
        }
        Ok(snapshot)
    }

    async fn insert(&self, params: &StrategyParams) -> Result<i32, RepoError> {
        let mut state = self.state.lock();
        if state.is_some() {
            return Err(RepoError::VersionConflict {
                entity: "strategy",
                id: params.strategy_id.clone(),
                version: 0,
            });
        }
        *state = Some(params.clone());
        Ok(1)
    }

    async fn update(&self, params: &StrategyParams) -> Result<bool, RepoError> {
        let mut state = self.state.lock();
        let Some(current) = state.as_ref() else {
            return Ok(false);
        };
        let mut updated = params.clone();
        updated.version = current.version + 1;
        *state = Some(updated);
        Ok(true)
    }

    async fn update_with_version(
        &self,
        _: &str,
        params: &StrategyParams,
        expected_version: i64,
    ) -> Result<bool, RepoError> {
        let mut state = self.state.lock();
        if state.as_ref().map(|current| current.version) != Some(expected_version) {
            return Ok(false);
        }
        let mut updated = params.clone();
        updated.version = expected_version + 1;
        *state = Some(updated);
        Ok(true)
    }

    async fn find_all(
        &self,
        _: Option<&str>,
        _: Option<StrategyType>,
        _: Option<StrategyStatus>,
        _: Option<bool>,
        _: i64,
        _: i64,
    ) -> Result<(Vec<StrategySummaryRow>, i64), RepoError> {
        unreachable!("save tests do not list strategies")
    }

    async fn count(
        &self,
        _: Option<&str>,
        _: Option<StrategyType>,
        _: Option<StrategyStatus>,
        _: Option<bool>,
    ) -> Result<i64, RepoError> {
        unreachable!("save tests do not count strategies")
    }

    async fn delete_by_id(&self, _: &str) -> Result<bool, RepoError> {
        unreachable!("save tests do not delete strategies")
    }

    async fn update_status(
        &self,
        _: &str,
        _: StrategyStatus,
        _: Option<&str>,
    ) -> Result<bool, RepoError> {
        unreachable!("save tests do not change lifecycle status")
    }

    async fn update_status_if(
        &self,
        _: &str,
        _: StrategyStatus,
        _: StrategyStatus,
        _: Option<&str>,
    ) -> Result<bool, RepoError> {
        unreachable!("save tests do not change lifecycle status")
    }

    async fn stats(&self) -> Result<StrategyStats, RepoError> {
        unreachable!("save tests do not query statistics")
    }
}

#[tokio::test]
async fn test_save_strategy_concurrent_updates_preserve_winner() {
    let mut existing = mock_strategy_params(StrategyStatus::Draft);
    existing.version = 7;
    let repo = Arc::new(SavingStrategyRepo::new(Some(existing.clone()), true));
    let svc = StrategyService::new(None, None, None, Some(repo.clone()), None);
    let mut first = existing.clone();
    first.strategy_name = "First edit".into();
    let mut second = existing;
    second.strategy_name = "Second edit".into();

    let (a, b) = tokio::join!(svc.save_strategy(&first), svc.save_strategy(&second));

    let winner = match (a, b) {
        (Ok(id), Err(ServiceError::Conflict(_))) if id == first.strategy_id => first,
        (Err(ServiceError::Conflict(_)), Ok(id)) if id == second.strategy_id => second,
        outcomes => panic!("expected one successful save and one conflict: {outcomes:?}"),
    };
    let stored = repo.state.lock();
    let stored = stored.as_ref().unwrap();
    assert_eq!(stored.strategy_name, winner.strategy_name);
    assert_eq!(stored.version, 8);
}

#[tokio::test]
async fn test_save_strategy_concurrent_inserts_preserve_winner() {
    let repo = Arc::new(SavingStrategyRepo::new(None, true));
    let svc = StrategyService::new(None, None, None, Some(repo.clone()), None);
    let mut first = mock_strategy_params(StrategyStatus::Draft);
    first.strategy_name = "First insert".into();
    let mut second = first.clone();
    second.strategy_name = "Second insert".into();

    let (a, b) = tokio::join!(svc.save_strategy(&first), svc.save_strategy(&second));

    let winner = match (a, b) {
        (Ok(_), Err(ServiceError::Conflict(_))) => first,
        (Err(ServiceError::Conflict(_)), Ok(_)) => second,
        outcomes => panic!("expected one successful insert and one conflict: {outcomes:?}"),
    };
    assert_eq!(
        repo.state.lock().as_ref().unwrap().strategy_name,
        winner.strategy_name
    );
}

#[tokio::test]
async fn test_save_and_update_strategy_share_version_guard() {
    let mut existing = mock_strategy_params(StrategyStatus::Draft);
    existing.version = 3;
    let repo = Arc::new(SavingStrategyRepo::new(Some(existing.clone()), true));
    let svc = StrategyService::new(None, None, None, Some(repo.clone()), None);
    let mut save = existing.clone();
    save.strategy_name = "Saved edit".into();
    let mut update = existing;
    update.strategy_name = "Updated edit".into();

    let (a, b) = tokio::join!(svc.save_strategy(&save), svc.update_strategy(&update));

    let winner = match (a, b) {
        (Ok(_), Err(ServiceError::Conflict(_))) => save,
        (Err(ServiceError::Conflict(_)), Ok(_)) => update,
        outcomes => panic!("expected one successful write and one conflict: {outcomes:?}"),
    };
    let stored = repo.state.lock();
    let stored = stored.as_ref().unwrap();
    assert_eq!(stored.strategy_name, winner.strategy_name);
    assert_eq!(stored.version, 4);
}

#[tokio::test]
async fn test_save_strategy_persists_defaults_without_overwriting_user_parameters() {
    let existing = mock_strategy_params(StrategyStatus::Draft);
    let repo = Arc::new(SavingStrategyRepo::new(Some(existing.clone()), false));
    let mut svc = StrategyService::new(None, None, None, Some(repo.clone()), None);
    svc.set_registry(Arc::new(strategy_engine::registry::default_registry()));

    svc.save_strategy(&existing).await.unwrap();
    let mut saved = svc
        .strategy_repo
        .as_ref()
        .unwrap()
        .find_by_id(&existing.strategy_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.params["lookback_period"], serde_json::json!(20));
    assert_eq!(saved.params["entry_threshold"], serde_json::json!(2.0));
    assert_eq!(saved.params["exit_threshold"], serde_json::json!(0.5));

    saved.params["lookback_period"] = serde_json::json!(50);
    saved.params["entry_threshold"] = serde_json::json!(3.0);
    saved
        .params
        .as_object_mut()
        .unwrap()
        .remove("exit_threshold");
    svc.save_strategy(&saved).await.unwrap();
    let saved = svc
        .strategy_repo
        .as_ref()
        .unwrap()
        .find_by_id(&existing.strategy_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.params["lookback_period"], serde_json::json!(50));
    assert_eq!(saved.params["entry_threshold"], serde_json::json!(3.0));
    assert_eq!(saved.params["exit_threshold"], serde_json::json!(0.5));
}

// ── create_strategy with defaults ──────────────────────────────────────

#[tokio::test]
async fn test_create_strategy_defaults_filled() {
    let mut mock_repo = MockStrategyRepo::new();

    // Expect insert with params that have schema defaults filled
    mock_repo
        .expect_insert()
        .withf(|params: &StrategyParams| {
            params.params.get("lookback_period") == Some(&serde_json::json!(20))
                && params.params.get("entry_threshold") == Some(&serde_json::json!(2.0))
                && params.params.get("exit_threshold") == Some(&serde_json::json!(0.5))
        })
        .returning(|_| Ok(1));

    let mut svc = make_mock_service(mock_repo, false);
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let result = svc
        .create_strategy(
            "MeanReversion",
            "Test Strategy",
            serde_json::json!({}),
            true,
            Decimal::from(10000),
            Decimal::from(500),
            None,
            None,
            vec![],
            vec!["BTC/USDT".to_string()],
            1,
        )
        .await;

    assert!(result.is_ok());
}

// ── create_strategy rejects unregistered types ────────────────────────────

#[tokio::test]
async fn test_create_strategy_rejects_unregistered_type() {
    let mock_repo = MockStrategyRepo::new();

    let mut svc = make_mock_service(mock_repo, false);
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    // The following are no longer valid StrategyType variants (removed as
    // unimplemented) and are not registered, so create_strategy must reject them.
    for unregistered in [
        "Statistical",
        "MachineLearning",
        "Custom",
        "Arbitrage",
        "MarketMaking",
    ] {
        let result = svc
            .create_strategy(
                unregistered,
                "Test Strategy",
                serde_json::json!({}),
                true,
                Decimal::from(10000),
                Decimal::from(500),
                None,
                None,
                vec![],
                vec![],
                1,
            )
            .await;

        assert!(
            result.is_err(),
            "type '{}' should be rejected",
            unregistered
        );
        assert!(
            matches!(result.unwrap_err(), ServiceError::NotFound(_)),
            "type '{}' should yield NotFound",
            unregistered
        );
    }
}

// ── get_strategy_type_info ─────────────────────────────────────────────

#[tokio::test]
async fn test_get_strategy_type_info_returns_metadata() {
    let mut svc = make_service_no_db();
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let result = svc.get_strategy_type_info("MeanReversion");
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.type_name, "MeanReversion");
    assert!(!info.parameters.is_empty());

    // Verify specific schema fields
    let lookback = info.parameters.iter().find(|p| p.name == "lookback_period");
    assert!(lookback.is_some());
    assert_eq!(lookback.unwrap().default, serde_json::json!(20));
}

#[tokio::test]
async fn test_get_strategy_type_info_unknown_type() {
    let mut svc = make_service_no_db();
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let result = svc.get_strategy_type_info("NonExistentType");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ServiceError::NotFound(_)));
}

// ── validate_strategy_params ──────────────────────────────────────────

#[tokio::test]
async fn test_validate_params_rejects_out_of_range() {
    let mut svc = make_service_no_db();
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let params = serde_json::json!({
        "lookback_period": 999,
        "entry_threshold": 2.0,
        "exit_threshold": 0.5,
    });
    let result = svc.validate_strategy_params("MeanReversion", &params);
    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::InvalidParameter(msg) => {
            assert!(
                msg.contains("out of range"),
                "Expected range error, got: {}",
                msg
            );
        }
        other => panic!("Expected InvalidParameter, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_validate_params_rejects_wrong_type() {
    let mut svc = make_service_no_db();
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let params = serde_json::json!({
        "lookback_period": "not-a-number",
        "entry_threshold": 2.0,
        "exit_threshold": 0.5,
    });
    let result = svc.validate_strategy_params("MeanReversion", &params);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_params_valid_params_pass() {
    let mut svc = make_service_no_db();
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let params = serde_json::json!({
        "lookback_period": 20,
        "entry_threshold": 2.0,
        "exit_threshold": 0.5,
    });
    let result = svc.validate_strategy_params("MeanReversion", &params);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_params_missing_required_field() {
    let mut svc = make_service_no_db();
    let registry = Arc::new(strategy_engine::registry::default_registry());
    svc.set_registry(registry);

    let params = serde_json::json!({
        "lookback_period": 20,
    });
    let result = svc.validate_strategy_params("MeanReversion", &params);
    assert!(result.is_err());
    match result.unwrap_err() {
        ServiceError::InvalidParameter(msg) => {
            assert!(
                msg.contains("Missing required parameter"),
                "Expected missing param error, got: {}",
                msg
            );
        }
        other => panic!("Expected InvalidParameter, got: {:?}", other),
    }
}

// ── Concurrent Lifecycle (PR2: TOCTOU fix) ────────────────────────────────

#[tokio::test]
async fn test_update_status_if_returns_false_on_condition_mismatch() {
    // CAS returning false must surface ConcurrentModification (not InvalidStatusTransition)
    let mut mock_repo = MockStrategyRepo::new();
    mock_repo
        .expect_find_by_id()
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Backtesting))));
    mock_repo
        .expect_update_status_if()
        .withf(|_id, _new, expected, _by| *expected == StrategyStatus::Backtesting)
        .returning(|_, _, _, _| Ok(false));

    let svc = make_mock_service(mock_repo, false);
    let result = svc.deploy_strategy("test_001").await;

    assert!(matches!(
        result.unwrap_err(),
        ServiceError::ConcurrentModification { ref strategy_id, expected: StrategyStatus::Backtesting }
        if strategy_id == "test_001"
    ));
}

#[tokio::test]
async fn test_deploy_strategy_concurrent_ac_a_simultaneous() {
    // AC-A (同时到达): tokio::join! two deploy requests; the shared mock
    // atomically grants "first writer wins" so one gets rows=1, the other rows=0.
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let wins = Arc::new(AtomicUsize::new(0));
    let wins_clone = wins.clone();
    let mut mock_repo = MockStrategyRepo::new();
    mock_repo
        .expect_find_by_id()
        .times(2)
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Backtesting))));
    mock_repo
        .expect_update_status_if()
        .times(2)
        .returning(move |_, _, _, _| Ok(wins_clone.fetch_add(1, Ordering::SeqCst) == 0));

    let svc = Arc::new(make_mock_service(mock_repo, false));
    let svc_a = svc.clone();
    let svc_b = svc.clone();
    let (res_a, res_b) = tokio::join!(
        async move { svc_a.deploy_strategy("test_001").await },
        async move { svc_b.deploy_strategy("test_001").await },
    );
    let results = [&res_a, &res_b];
    let oks = results.iter().filter(|r| r.is_ok()).count();
    let conflicts = results
        .iter()
        .filter(|r| matches!(r, Err(ServiceError::ConcurrentModification { .. })))
        .count();
    assert_eq!(oks, 1, "results: {:?} / {:?}", res_a, res_b);
    assert_eq!(conflicts, 1, "results: {:?} / {:?}", res_a, res_b);
}

#[tokio::test]
async fn test_deploy_strategy_concurrent_ac_b_sequential() {
    // AC-B (顺序到达): first request moved the row to Deployed; the second
    // reads Deployed, finds Deployed→Deployed is not valid, and surfaces
    // InvalidStatusTransition (state moved on, not a race loss).
    let mut mock_repo = MockStrategyRepo::new();
    mock_repo
        .expect_find_by_id()
        .returning(|_| Ok(Some(mock_strategy_params(StrategyStatus::Deployed))));

    let svc = make_mock_service(mock_repo, false);
    let result = svc.deploy_strategy("test_001").await;

    assert!(matches!(
        result.unwrap_err(),
        ServiceError::InvalidStatusTransition { ref from, ref to }
        if from == "Deployed" && to == "Deployed"
    ));
}
