# 发布说明 / Release Note

> CHANGELOG 风格，可直接追加到 `CHANGELOG.md`。建议版本号：`0.2.1`（refactor，无功能增删）。

```
## [0.2.1] - 2026-08-22

### Changed

- **架构重构（遵循软件设计 7 原则）**：在不改变外部行为的前提下，系统性收敛设计债，净减少约 494 行。
  - **命令层薄壳**：新增 `OrderProcessor` 用例承载完整下单编排（行情→风控→提交→持久化→事件→异步执行）；
    `submit_order` 由 ~110 行「上帝函数」变为薄适配器（SRP / 分层 / DIP）。
  - **装配收敛**：引入 `SharedInfra` 打包注入，`AppServices` 构造参数由 10 → 2；消除 `OkxExecutor` 重复实例化，
    命令层与服务层共享同一 `Arc<OkxExecutor>`（DIP / DRY）。
  - **分层加固**：`get_market_data` / `get_okx_realtime_data` / `get_okx_historical_data` 改经 `market_service`，
    命令层不再直接依赖 `data_layer`。
  - **前端 DRY**：合并 `useFormat` → 权威 `useFormatting`；删除死代码 `MetricCard.vue`。
  - **前端 SoC**：拆分 `services/market` + `ws`、`services/okx` + `okxOrder`。
  - **前端 SRP**：`stores/strategy` 拆分为 `strategy`（数据/CRUD/类型/轮询）+ `strategyLifecycle`（生命周期动作，
    组合 base store）。
  - **前端 DIP**：新增 `transport.ts`，服务层不再直接 `import @tauri-apps/api`，框架依赖收敛到单点。

### Tests

- 验证：`cargo test --workspace` 559 passed / 0 failed / 17 ignored；`cargo clippy --all-targets` 0 warning；
  `vue-tsc` 通过；`npm test` 34 文件 / 426 passed。
- 外部行为零变化（测试基线一致，仅新增 `strategyLifecycle.store.test.ts`）。

### Notes

- `pre_trade_check` / `get_risk_metrics` 保留 `risk_layer` 直连，属**有意风控领域边界**
  （需返回具体失败原因并触发告警）；经当前 `risk_service.pre_trade_check` 会丢失失败明细，进阶需先增强 service 签名。
```

## 合并版 PR + Commit 摘要
（也可直接作为 PR 描述 / release note 正文，`PR.md` + `COMMIT_MESSAGE.md` 合并）

**PR 标题**：`refactor: apply SOLID + DRY/KISS + SoC across backend & frontend`

**提交序列**：
```
5938d2d refactor(services): extract OrderProcessor use-case & bundle infra via SharedInfra
075301b refactor(frontend): DRY/SoC/SRP store & service split + IPC transport
fff8263 refactor(commands): route market-data commands through market_service
```

**一句话总结**：命令层薄壳化、服务层唯一编排点、装配依赖束（SharedInfra）、前端 DRY/SoC/SRP 拆分与 IPC 抽象；净 −494 行，测试基线一致（Rust 559 / 前端 426）。
