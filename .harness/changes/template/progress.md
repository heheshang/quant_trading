# 进度追踪文件 — 会话粒度

> **职责分离：** progress.md 追踪**会话内**的实时进度（当前阶段 + 任务状态 + 阻塞项）。
> **阶段完成状态**归 summary.md 管理（`summary.md` 是 Single Source of Truth）。
> progress.md 用于跨 Session 的恢复，summary.md 用于交付后的归档。

## 基本信息

- **需求名称**：{需求名称}
- **创建日期**：{YYYY-MM-DD}
- **当前阶段**：{阶段编号 - 阶段名称}
- **最后更新**：{YYYY-MM-DD HH:mm:ss}

## 当前会话阶段

| 阶段 | 状态 | 开始时间 | 当前操作 |
|------|------|----------|----------|
| {阶段名} | {in_progress/pending} | {datetime} | {正在做什么} |

> 阶段完成后的状态迁移请更新到 `summary.md`，此处只记录"当前在哪一步"。

## 任务进度

### {任务标题}
- **状态**：{pending/in_progress/completed/blocked}
- **阻塞原因**：{reason}（如状态为 blocked）
- **完成时间**：{datetime}

## 阻塞记录

| 时间 | 阻塞原因 | 解决状态 | 解决方案 |
|------|----------|----------|----------|
| {datetime} | {reason} | {open/resolved} | {solution} |

## 恢复指令

```
# 新会话启动时执行（在 owner-agent.md 第零节启动仪式之后）
1. 读取本文件 → 了解当前在哪里
2. 读取 summary.md → 了解已完成到哪一步
3. 加载对应阶段的 L2 Skill
4. 继续
```
