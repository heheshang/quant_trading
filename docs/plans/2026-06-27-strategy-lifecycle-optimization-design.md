# Strategy Lifecycle Optimization Design

**Date**: 2026-06-27
**Scope**: Full lifecycle optimization (Repository → Service → Strategy → Scheduler → Commands)
**Approach**: Incremental Layer-by-Layer

---

## Problem Statement

The strategy management chain has significant gaps:

| Area | Issue |
|------|-------|
| Repository | N+1 queries, no batch operations, no transactions |
| Service | No validation, no error rollbacks, incomplete lifecycle logic |
| Strategy | Empty lifecycle hooks (`on_deploy()`, `on_start()`, etc.) |
| Scheduler | No health checks, no monitoring metrics |
| Commands | No input validation, inconsistent error mapping |

---

## Design

### Phase 1: Repository Layer

**Goal**: Fix N+1 queries, add batch operations, transactions.

**Changes**:

1. `find_all_with_details()` — Single query with JOIN to get strategy + last backtest + running status
2. `batch_update_status()` — Atomic status updates for multiple strategies
3. `transaction_guard()` — Transaction wrapper for multi-step operations

**Before**:
```rust
let strategies = repo.find_all().await?;
for s in &strategies {
    let backtest = repo.get_last_backtest(&s.id).await?; // N queries
}
```

**After**:
```rust
let strategies = repo.find_all_with_details().await?;
// Each strategy includes: last_backtest, running_status, signal_count
```

---

### Phase 2: Service Layer

**Goal**: Add validation, error rollbacks, lifecycle logic.

**Changes**:

1. `validate_strategy_params()` — Validate before any lifecycle transition
2. `lifecycle_transition()` — Atomic status change with rollback on failure
3. `StrategyLifecycleError` — Typed errors for each failure mode

**Lifecycle State Machine**:
```
Draft → Backtesting → Deployed → Running ⇄ Paused → Archived
                     ↓
                   Draft (rollback)
```

**Validation Rules**:
- `Draft → Backtesting`: params must have valid symbol, lookback_period > 0
- `Backtesting → Deployed`: must have successful backtest result
- `Deployed → Running`: must pass risk checks
- `Running → Paused`: must have no open positions
- `Paused → Running`: must pass risk checks again

---

### Phase 3: Strategy Layer

**Goal**: Implement lifecycle hooks with real logic.

| Hook | Implementation |
|------|----------------|
| `on_deploy()` | Initialize state, validate params, log deployment |
| `on_start()` | Reset counters, connect to market data, start signal generation |
| `on_stop()` | Flush pending signals, disconnect, persist state |
| `on_pause()` | Suspend signal generation, keep connections alive |
| `on_resume()` | Resume signal generation, revalidate state |
| `on_archive()` | Cleanup resources, persist final state, remove from active set |

---

### Phase 4: Scheduler Layer

**Goal**: Add health checks, monitoring metrics.

**Changes**:

1. `health_check()` — Periodic health verification for running strategies
2. `metrics_collector()` — Collect signal_count, error_rate, latency
3. `StrategyMetrics` — Metrics struct for monitoring

**Health Check Logic**:
```rust
// Every 60 seconds
if strategy.last_signal_time + timeout < now {
    warn!("Strategy {} appears stalled", strategy_id);
    circuit_breaker.record_error();
}
```

---

### Phase 5: Commands Layer

**Goal**: Add input validation, error mapping.

**Changes**:

1. `validate_command_input()` — Validate all strategy command inputs
2. `map_lifecycle_error()` — Convert ServiceError → user-friendly Tauri error
3. `StrategyCommandResult` — Standardized command response type

**Error Response Format**:
```json
{
  "success": false,
  "error": {
    "code": "INVALID_STATUS_TRANSITION",
    "message": "Cannot start strategy in Archived status",
    "details": { "current_status": "Archived", "target_status": "Running" }
  }
}
```

---

## Success Criteria

- [ ] Zero N+1 queries in strategy listing
- [ ] All lifecycle transitions validated before execution
- [ ] All lifecycle hooks implemented with real logic
- [ ] Health checks running for all active strategies
- [ ] Consistent error responses from all Tauri commands
- [ ] All existing tests pass
- [ ] New tests for each phase

---

## Out of Scope

- Strategy versioning (future enhancement)
- Strategy templates (future enhancement)
- Real-time market data integration (existing)
- Risk management integration (existing)
