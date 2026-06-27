# Strategy Lifecycle Optimization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Optimize the strategy management full chain — fix N+1 queries, add validation, implement lifecycle hooks, add health checks, and standardize error responses.

**Architecture:** Incremental layer-by-layer approach: Repository → Service → Strategy → Scheduler → Commands. Each phase is independently testable.

**Tech Stack:** Rust, sqlx (PostgreSQL), tokio, chrono, serde, thiserror

---

## Phase 1: Repository Layer

### Task 1.1: Add `find_all_with_details()` to StrategyRepository

**Files:**
- Modify: `crates/repository/src/strategy.rs:84-110` (trait)
- Modify: `crates/repository/src/strategy.rs:124-352` (impl)
- Test: `crates/repository/src/strategy.rs` (add test module)

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "../data-layer/migrations")]
    async fn test_find_all_with_details_returns_empty(pool: PgPool) {
        let repo = PgStrategyRepository::new(pool);
        let result = repo.find_all_with_details().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p quant-repository -- test_find_all_with_details`
Expected: FAIL with "method not found"

**Step 3: Add trait method**

```rust
// In StrategyRepository trait
async fn find_all_with_details(&self) -> RepositoryResult<Vec<StrategyWithDetails>>;
```

**Step 4: Add struct**

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StrategyWithDetails {
    pub strategy: StrategyParams,
    pub last_backtest_id: Option<String>,
    pub last_backtest_status: Option<String>,
    pub is_running: bool,
    pub signal_count: Option<i64>,
}
```

**Step 5: Implement with JOIN query**

```rust
async fn find_all_with_details(&self) -> RepositoryResult<Vec<StrategyWithDetails>> {
    let rows = sqlx::query_as::<_, (StrategyParams, Option<String>, Option<String>, bool, Option<i64>)>(
        r#"
        SELECT 
            s.*,
            b.id as last_backtest_id,
            b.status as last_backtest_status,
            (sc.strategy_id IS NOT NULL) as is_running,
            sc.signal_count
        FROM strategies s
        LEFT JOIN LATERAL (
            SELECT id, status 
            FROM backtests 
            WHERE strategy_id = s.id 
            ORDER BY created_at DESC 
            LIMIT 1
        ) b ON true
        LEFT JOIN scheduler_tasks sc ON sc.strategy_id = s.id
        ORDER BY s.created_at DESC
        "#
    )
    .fetch_all(&self.pool)
    .await
    .map_err(|e| RepositoryError::Database(e))?;

    Ok(rows.into_iter().map(|(strategy, last_backtest_id, last_backtest_status, is_running, signal_count)| {
        StrategyWithDetails {
            strategy,
            last_backtest_id,
            last_backtest_status,
            is_running,
            signal_count,
        }
    }).collect())
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test -p quant-repository -- test_find_all_with_details`
Expected: PASS

**Step 7: Commit**

```bash
git add crates/repository/src/strategy.rs
git commit -m "feat(repository): add find_all_with_details with JOIN query"
```

---

### Task 1.2: Add `batch_update_status()` to StrategyRepository

**Files:**
- Modify: `crates/repository/src/strategy.rs:84-110` (trait)
- Modify: `crates/repository/src/strategy.rs:124-352` (impl)
- Test: `crates/repository/src/strategy.rs`

**Step 1: Write the failing test**

```rust
#[sqlx::test(migrations = "../data-layer/migrations")]
async fn test_batch_update_status(pool: PgPool) {
    let repo = PgStrategyRepository::new(pool);
    // Insert test strategies first
    // ...
    let result = repo.batch_update_status(&["id1", "id2"], StrategyStatus::Running).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p quant-repository -- test_batch_update_status`
Expected: FAIL

**Step 3: Add trait method**

```rust
async fn batch_update_status(
    &self,
    strategy_ids: &[&str],
    status: StrategyStatus,
) -> RepositoryResult<u64>;
```

**Step 4: Implement**

```rust
async fn batch_update_status(
    &self,
    strategy_ids: &[&str],
    status: StrategyStatus,
) -> RepositoryResult<u64> {
    if strategy_ids.is_empty() {
        return Ok(0);
    }
    
    let status_str = status.to_string();
    let ids: Vec<&str> = strategy_ids.to_vec();
    
    let result = sqlx::query(
        "UPDATE strategies SET status = $1, updated_at = NOW() WHERE id = ANY($2)"
    )
    .bind(&status_str)
    .bind(&ids)
    .execute(&self.pool)
    .await
    .map_err(|e| RepositoryError::Database(e))?;

    Ok(result.rows_affected())
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p quant-repository -- test_batch_update_status`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/repository/src/strategy.rs
git commit -m "feat(repository): add batch_update_status for atomic updates"
```

---

## Phase 2: Service Layer

### Task 2.1: Add `validate_strategy_params()` to StrategyService

**Files:**
- Modify: `crates/services/src/strategy_service.rs:16-24` (struct)
- Modify: `crates/services/src/error.rs` (add error variant)
- Test: `crates/services/src/strategy_service.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_validate_strategy_params_invalid_symbol() {
    let service = StrategyService::new(None, None, None);
    let params = StrategyParams {
        strategy_id: "test".to_string(),
        strategy_name: "Test".to_string(),
        strategy_type: StrategyType::MeanReversion,
        params: serde_json::json!({"symbol": ""}),
        // ...
    };
    let result = service.validate_strategy_params(&params);
    assert!(result.is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p quant-services -- test_validate_strategy_params`
Expected: FAIL

**Step 3: Add error variant**

```rust
// In crates/services/src/error.rs
pub enum ServiceError {
    // ... existing variants
    Validation(String),
}
```

**Step 4: Implement validation**

```rust
impl StrategyService {
    pub fn validate_strategy_params(&self, params: &StrategyParams) -> ServiceResult<()> {
        if params.strategy_name.is_empty() {
            return Err(ServiceError::Validation("Strategy name cannot be empty".into()));
        }
        
        let symbol = params.params.get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        if symbol.is_empty() {
            return Err(ServiceError::Validation("Symbol cannot be empty".into()));
        }
        
        let lookback = params.params.get("lookback_period")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        
        if lookback == 0 {
            return Err(ServiceError::Validation("Lookback period must be > 0".into()));
        }
        
        Ok(())
    }
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p quant-services -- test_validate_strategy_params`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/services/src/strategy_service.rs crates/services/src/error.rs
git commit -m "feat(service): add validate_strategy_params with typed errors"
```

---

### Task 2.2: Add `lifecycle_transition()` with rollback

**Files:**
- Modify: `crates/services/src/strategy_service.rs`
- Test: `crates/services/src/strategy_service.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_lifecycle_transition_rollback_on_failure() {
    // Test that if on_start() fails, status rolls back to Deployed
    // ...
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p quant-services -- test_lifecycle_transition_rollback`
Expected: FAIL

**Step 3: Implement**

```rust
impl StrategyService {
    async fn lifecycle_transition(
        &self,
        strategy_id: &str,
        from: StrategyStatus,
        to: StrategyStatus,
        hook: impl FnOnce(&mut dyn Strategy) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
    ) -> ServiceResult<StrategyStatus> {
        let repo = self.strategy_repo.as_ref().ok_or(ServiceError::DatabaseNotConnected)?;
        
        // 1. Fetch and validate current status
        let mut params = repo.find_by_id(strategy_id).await
            .map_err(|e| ServiceError::Other(e.to_string()))?
            .ok_or_else(|| ServiceError::NotFound(format!("Strategy '{}' not found", strategy_id)))?;
        
        if params.status != from {
            return Err(ServiceError::InvalidStatusTransition { from: params.status, to });
        }
        
        // 2. Build strategy and run hook
        let (_, mut strategy) = self.build_strategy_from_params(&params).await?;
        hook(&mut *strategy).await
            .map_err(|e| ServiceError::Strategy(e.to_string()))?;
        
        // 3. Persist status
        params.status = to;
        params.updated_at = chrono::Utc::now();
        repo.update(&params).await
            .map_err(|e| ServiceError::Other(e.to_string()))?;
        
        Ok(to)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p quant-services -- test_lifecycle_transition_rollback`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/services/src/strategy_service.rs
git commit -m "feat(service): add lifecycle_transition with rollback support"
```

---

## Phase 3: Strategy Layer

### Task 3.1: Implement `on_deploy()` hook

**Files:**
- Modify: `crates/strategy-layer/src/strategy.rs:44-71`
- Test: `crates/strategy-layer/src/strategy.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_on_deploy_initializes_state() {
    let mut strategy = MeanReversionStrategy::new();
    let params = StrategyParams { /* valid params */ };
    strategy.initialize(params).await.unwrap();
    
    let result = strategy.on_deploy().await;
    assert!(result.is_ok());
    // Verify state was initialized
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p strategy-layer -- test_on_deploy`
Expected: FAIL (or passes but doesn't actually do anything)

**Step 3: Implement**

```rust
async fn on_deploy(&mut self) -> Result<()> {
    // Validate required parameters
    let _symbol = self.params.as_ref()
        .and_then(|p| p.params.get("symbol"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| StrategyError::InvalidParams("Missing symbol parameter".into()))?;
    
    let _lookback = self.params.as_ref()
        .and_then(|p| p.params.get("lookback_period"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| StrategyError::InvalidParams("Missing lookback_period parameter".into()))?;
    
    // Initialize internal state
    self.signals.clear();
    self.position = Decimal::ZERO;
    self.unrealized_pnl = Decimal::ZERO;
    
    info!("Strategy deployed with validated parameters");
    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p strategy-layer -- test_on_deploy`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/strategy-layer/src/strategy.rs
git commit -m "feat(strategy): implement on_deploy hook with validation"
```

---

### Task 3.2: Implement remaining lifecycle hooks

**Files:**
- Modify: `crates/strategy-layer/src/strategy.rs:44-71`
- Test: `crates/strategy-layer/src/strategy.rs`

**Step 1: Write failing tests for each hook**

```rust
#[tokio::test]
async fn test_on_start_resets_counters() { /* ... */ }

#[tokio::test]
async fn test_on_stop_flushes_signals() { /* ... */ }

#[tokio::test]
async fn test_on_pause_suspends_generation() { /* ... */ }

#[tokio::test]
async fn test_on_resume_revalidates() { /* ... */ }

#[tokio::test]
async fn test_on_archive_cleans_up() { /* ... */ }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p strategy-layer -- test_on_start test_on_stop test_on_pause test_on_resume test_on_archive`
Expected: FAIL

**Step 3: Implement each hook**

```rust
async fn on_start(&mut self) -> Result<()> {
    self.signal_count = 0;
    self.error_count = 0;
    self.last_signal_time = None;
    info!("Strategy started, counters reset");
    Ok(())
}

async fn on_stop(&mut self) -> Result<()> {
    // Flush any pending signals
    self.signals.clear();
    info!("Strategy stopped, signals flushed");
    Ok(())
}

async fn on_pause(&mut self) -> Result<()> {
    // Signal generation will be suspended by scheduler
    info!("Strategy paused");
    Ok(())
}

async fn on_resume(&mut self) -> Result<()> {
    // Revalidate state
    if self.params.is_none() {
        return Err(StrategyError::NotInitialized);
    }
    info!("Strategy resumed");
    Ok(())
}

async fn on_archive(&mut self) -> Result<()> {
    // Cleanup resources
    self.signals.clear();
    self.position = Decimal::ZERO;
    info!("Strategy archived, resources cleaned up");
    Ok(())
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p strategy-layer -- test_on_start test_on_stop test_on_pause test_on_resume test_on_archive`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/strategy-layer/src/strategy.rs
git commit -m "feat(strategy): implement all lifecycle hooks"
```

---

## Phase 4: Scheduler Layer

### Task 4.1: Add health check to scheduler

**Files:**
- Modify: `crates/strategy-layer/src/scheduler/mod.rs`
- Modify: `crates/strategy-layer/src/scheduler/task.rs`
- Test: `crates/strategy-layer/src/scheduler/mod.rs`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_health_check_detects_stalled_strategy() {
    // Test that health check detects strategies that haven't produced signals
    // ...
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p strategy-layer -- test_health_check`
Expected: FAIL

**Step 3: Add `last_signal_time` to SchedulerTaskMeta**

```rust
// In task.rs
pub struct SchedulerTaskMeta {
    pub strategy_name: String,
    pub interval_secs: u64,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_signal_time: Option<DateTime<Utc>>,
    pub error_count: u32,
}
```

**Step 4: Implement health check**

```rust
// In mod.rs
pub async fn health_check(&self) -> Vec<StrategyHealth> {
    let tasks = self.tasks.read().await;
    let mut health_status = Vec::new();
    
    for (id, handle) in tasks.iter() {
        let meta = handle.meta.lock().unwrap();
        let error_count = handle.error_counter.load(Ordering::Acquire);
        
        let health = StrategyHealth {
            strategy_id: id.clone(),
            strategy_name: meta.strategy_name.clone(),
            last_run_at: meta.last_run_at,
            error_count,
            is_healthy: error_count < self.config.circuit_breaker_threshold,
        };
        
        health_status.push(health);
    }
    
    health_status
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p strategy-layer -- test_health_check`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/strategy-layer/src/scheduler/mod.rs crates/strategy-layer/src/scheduler/task.rs
git commit -m "feat(scheduler): add health_check for strategy monitoring"
```

---

## Phase 5: Commands Layer

### Task 5.1: Add input validation to Tauri commands

**Files:**
- Modify: `src-tauri/src/commands.rs:534-706`
- Test: `src-tauri/src/commands.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_validate_strategy_command_input_empty_name() {
    let input = StrategyInput {
        name: "".to_string(),
        // ...
    };
    let result = validate_strategy_input(&input);
    assert!(result.is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p quant-tauri -- test_validate_strategy_command_input`
Expected: FAIL

**Step 3: Implement validation**

```rust
fn validate_strategy_input(input: &StrategyInput) -> Result<(), String> {
    if input.name.is_empty() {
        return Err("Strategy name cannot be empty".into());
    }
    
    if input.symbol.is_empty() {
        return Err("Symbol cannot be empty".into());
    }
    
    if input.lookback_period == 0 {
        return Err("Lookback period must be > 0".into());
    }
    
    Ok(())
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p quant-tauri -- test_validate_strategy_command_input`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): add input validation for strategy commands"
```

---

### Task 5.2: Standardize error responses

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/commands.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_map_lifecycle_error_invalid_transition() {
    let error = ServiceError::InvalidStatusTransition {
        from: StrategyStatus::Archived,
        to: StrategyStatus::Running,
    };
    let result = map_lifecycle_error(error);
    assert_eq!(result.code, "INVALID_STATUS_TRANSITION");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p quant-tauri -- test_map_lifecycle_error`
Expected: FAIL

**Step 3: Implement error mapping**

```rust
#[derive(Serialize)]
struct CommandError {
    code: String,
    message: String,
    details: Option<serde_json::Value>,
}

fn map_lifecycle_error(error: ServiceError) -> CommandError {
    match error {
        ServiceError::InvalidStatusTransition { from, to } => CommandError {
            code: "INVALID_STATUS_TRANSITION".into(),
            message: format!("Cannot transition from {:?} to {:?}", from, to),
            details: Some(serde_json::json!({
                "current_status": from,
                "target_status": to,
            })),
        },
        ServiceError::NotFound(msg) => CommandError {
            code: "NOT_FOUND".into(),
            message: msg,
            details: None,
        },
        ServiceError::Validation(msg) => CommandError {
            code: "VALIDATION_ERROR".into(),
            message: msg,
            details: None,
        },
        _ => CommandError {
            code: "INTERNAL_ERROR".into(),
            message: "An internal error occurred".into(),
            details: None,
        },
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p quant-tauri -- test_map_lifecycle_error`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): standardize error responses for strategy commands"
```

---

## Final Verification

### Task F.1: Run full test suite

**Step 1: Run all tests**

Run: `cargo test --workspace --exclude exchange-okx`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy --workspace --exclude exchange-okx -- -D warnings`
Expected: No warnings

**Step 3: Run formatter**

Run: `cargo fmt --check`
Expected: No formatting issues

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: verify all tests pass, no clippy warnings, formatting clean"
```
