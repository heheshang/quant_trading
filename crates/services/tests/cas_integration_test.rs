use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use quant_domain::types::{StrategyParams, StrategyStatus, StrategyType};
use data_layer::{RepoError, StrategyRepository, StrategyStats, StrategySummaryRow};

struct MockStrategyRepo {
    state: Arc<Mutex<HashMap<String, StrategyStatus>>>,
}

impl MockStrategyRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    async fn seed(&self, id: &str, status: StrategyStatus) {
        self.state.lock().await.insert(id.to_string(), status);
    }
}

fn stub<T>(msg: &str) -> Result<T, RepoError> {
    Err(RepoError::Database(msg.to_string()))
}

#[async_trait]
impl StrategyRepository for MockStrategyRepo {
    async fn find_all(
        &self,
        _: Option<&str>,
        _: Option<StrategyType>,
        _: Option<StrategyStatus>,
        _: Option<bool>,
        _: i64,
        _: i64,
    ) -> Result<(Vec<StrategySummaryRow>, i64), RepoError> {
        stub("not used in CAS test")
    }
    async fn count(
        &self,
        _: Option<&str>,
        _: Option<StrategyType>,
        _: Option<StrategyStatus>,
        _: Option<bool>,
    ) -> Result<i64, RepoError> {
        stub("not used in CAS test")
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<StrategyParams>, RepoError> {
        Ok(None)
    }
    async fn insert(&self, _: &StrategyParams) -> Result<i32, RepoError> {
        stub("not used in CAS test")
    }
    async fn update(&self, _: &StrategyParams) -> Result<bool, RepoError> {
        stub("not used in CAS test")
    }
    async fn delete_by_id(&self, _: &str) -> Result<bool, RepoError> {
        stub("not used in CAS test")
    }
    async fn update_status(
        &self,
        _: &str,
        _: StrategyStatus,
        _: Option<&str>,
    ) -> Result<bool, RepoError> {
        stub("not used in CAS test")
    }
    async fn update_with_version(
        &self,
        _: &str,
        _: &StrategyParams,
        _: i64,
    ) -> Result<bool, RepoError> {
        stub("not used in CAS test")
    }
    async fn update_status_if(
        &self,
        id: &str,
        new: StrategyStatus,
        expected: StrategyStatus,
        _: Option<&str>,
    ) -> Result<bool, RepoError> {
        let mut s = self.state.lock().await;
        match s.get(id).copied() {
            Some(cur) if cur == expected => {
                s.insert(id.to_string(), new);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    async fn stats(&self) -> Result<StrategyStats, RepoError> {
        stub("not used in CAS test")
    }
}

#[tokio::test]
#[ignore = "PR8 nightly-only integration test (real SQL CAS semantics)"]
async fn test_update_status_if_concurrent_only_one_wins() {
    let repo: Arc<dyn StrategyRepository> = {
        let r = MockStrategyRepo::new();
        r.seed("strat_cas_001", StrategyStatus::Draft).await;
        Arc::new(r)
    };
    let a = Arc::clone(&repo);
    let b = Arc::clone(&repo);
    let (res_a, res_b) = tokio::join!(
        async move {
            a.update_status_if(
                "strat_cas_001",
                StrategyStatus::Running,
                StrategyStatus::Draft,
                None,
            )
            .await
        },
        async move {
            b.update_status_if(
                "strat_cas_001",
                StrategyStatus::Running,
                StrategyStatus::Draft,
                None,
            )
            .await
        },
    );
    let oks = [&res_a, &res_b]
        .iter()
        .filter(|r| matches!(r, Ok(true)))
        .count();
    let losers = [&res_a, &res_b]
        .iter()
        .filter(|r| matches!(r, Ok(false)))
        .count();
    assert_eq!(
        oks, 1,
        "expected exactly one CAS winner, got: {:?} / {:?}",
        res_a, res_b
    );
    assert_eq!(
        losers, 1,
        "expected exactly one CAS loser, got: {:?} / {:?}",
        res_a, res_b
    );
}
