# 提交信息记录（COMMIT_MESSAGE）

> 本次架构重构已按方案 A 拆分为 3 个逻辑提交（基于 `8dbdeea`）。以下为实际执行的提交信息，可用于 CHANGELOG / release note。

## COMMIT ①
```
refactor(services): extract OrderProcessor use-case & bundle infra via SharedInfra

- Add OrderProcessor use-case in services: market data -> risk check ->
  submit -> persist -> event -> async execution (SRP/DIP). submit_order
  becomes a thin command adapter instead of a ~110-line handler.
- Introduce SharedInfra to bundle composition-root deps; AppServices
  constructors drop from 10 to 2 args.
- Share a single Arc<OkxExecutor> between AppState and AppServices
  (removes a duplicated instance). Align AppState.okx_executor type.
```

## COMMIT ②
```
refactor(frontend): DRY/SoC/SRP store & service split + IPC transport

- DRY: merge useFormat into canonical useFormatting; remove dead MetricCard.
- SoC: split services into market/ws and okx/okxOrder.
- SRP: split strategy store into strategy + strategyLifecycle.
- DIP: add transport.ts so services no longer import @tauri-apps/api.
- Migrate tests (add strategyLifecycle.store.test.ts).
```

## COMMIT ③
```
refactor(commands): route market-data commands through market_service

- get_market_data / get_okx_realtime_data / get_okx_historical_data now
  delegate to market_service.get_realtime_data / get_historical_data, so
  the command layer no longer touches data-layer directly (layering/DIP).
  OKX observability metrics stay in the command adapter.
- Update commands tests to the layered not-initialized message.
```

## 实际生成的历史
```
fff8263 refactor(commands): route market-data commands through market_service
075301b refactor(frontend): DRY/SoC/SRP store & service split + IPC transport
5938d2d refactor(services): extract OrderProcessor use-case & bundle infra via SharedInfra
8dbdeea test(trading-layer): cover OkxExecutionStrategy success & fallback via mock
```
