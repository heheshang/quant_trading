# 设计产物归档说明（ARCHIVE）

> 本变更的设计产物归档索引。说明每个文件的用途、如何复用，以及它们与本 PR 的关系。
> **注意**：本目录（`.harness/changes/redesign-7principles-20260822/`）按 Harness 设计属**本地工作流产物**，被 `.harness/.gitignore` 忽略，不入 git；PR 描述请从 `PR_DESCRIPTION.md` 复制。

## 产物清单

| 文件 | 用途 | 说明 |
|------|------|------|
| `design.md` | 设计蓝图 | 7 原则审计（AS-IS 违规表）+ TO-BE 目标架构 + **2.2 AS-IS→TO-BE 架构迁移总览图**（4 张） |
| `spec.md` | 需求规格 | 背景/范围/影响分析/验收标准（含"命令层不直连基础设施"验收项） |
| `tasks.md` | 任务分解 | 按 B1～B6 批次编排，含完成状态 |
| `summary.md` | 变更摘要（SSOT） | 批次执行状态、验证结果、变更文件清单、已知例外 |
| `PR_DESCRIPTION.md` | PR/提交描述 | 可整个复制粘贴到 GitHub PR 描述框（见下方） |
| `contract.md` / `progress.md` / `review.md` / `feature_list.json` / `test-plan.md` / `deploy-log.md` | 模板占位 | 来自 `template/`，若用 Harness 双 Agent 流程可逐步填充 |

## 设计原则 → 产物映射

| 原则 | 依据 |
|------|------|
| SRP / DIP / DRY / SoC / YAGNI/KISS | `design.md` §1 审计表 + §2 目标架构 + §2.2 迁移图 |
| 分层（命令层零 data_layer 直连） | `design.md` §2 / `tasks.md` B5 / `summary.md` 分层加固段 |

## 与提交历史的关系

| 提交 | 对应产物 |
|------|----------|
| ① `refactor(services): OrderProcessor + SharedInfra` | `design.md` §2.2 装配收敛图、B2/B5 |
| ② `refactor(frontend): DRY/SoC/SRP + transport` | `design.md` §2.2 前端模块拆分图、B1/B3/B4/B4-2/B6 |
| ③ `refactor(commands): route market-data through services` | `design.md` §2.2 TO-BE 图、B5 分层加固 |

## 复用指引

- **想了解为什么这样改** → 读 `design.md` §1（现状审计）。
- **想对照验收标准** → 读 `spec.md`（验收标准）与 `tasks.md`。
- **想评审/复现** → 跑 `cargo test --workspace`（559）与 `npm test`（426）。
- **想开 PR** → 用下方 `PR_DESCRIPTION.md` 内容。
