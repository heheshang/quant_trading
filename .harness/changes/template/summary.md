# 变更摘要

> 整个变更的 Single Source of Truth。记录每个阶段的执行状态、评审结论和例外情况。

## 基本信息

- **需求名称**：{需求名称}
- **变更类型**：{feat / fix / refactor / docs / chore}
- **日期**：{YYYYMMDD}
- **Owner**：Application Owner Agent

## 阶段执行状态

| 阶段 | 范围 | 状态 | 轮次 | 备注 |
|------|------|------|------|------|
| 需求分析 | Core | ⬜ | - | |
| 需求评审 | Core | ⬜ | - | |
| 编码实现 | Core | ⬜ | - | |
| 编码评审 | Core | ⬜ | - | |
| 单元测试编写 | Core | ⬜ | - | |
| 单元测试 CI | Core | ⬜ | - | |
| 集成测试 | Extended | ⬜ | - | |
| 部署验证 | Extended | ⬜ | - | |
| 灰度发布 | Extended | ⬜ | - | |
| 交付确认 | Extended | ⬜ | - | |

## 评审记录

| 评审类型 | 轮次 | 结论 | MUST FIX | LOW | INFO |
|----------|------|------|----------|-----|------|
| 需求评审 | 1 | {通过/需修改} | {n} | {n} | {n} |
| 编码评审 | 1 | {通过/需修改} | {n} | {n} | {n} |
| 测试评审 | 1 | {通过/需修改} | {n} | {n} | {n} |

## 变更文件清单

| 文件路径 | 变更类型 | 说明 |
|----------|----------|------|
| {crates/my-service/src/services/xxx_service.rs} | {新增/修改/删除} | {变更说明} |
| {crates/my-service/src/services/default_xxx_service.rs} | {新增/修改/删除} | {变更说明} |
| {crates/my-service/tests/xxx_service_test.rs} | {新增/修改/删除} | {变更说明} |
| {crates/my-service/src/repositories/xxx_repository.rs} | {新增/修改/删除} | {变更说明} |

## CI 信息

- **构建编号**：#{CI build number}
- **测试用例数**：{total} / **通过**：{passed} / **失败**：{failed}
- **代码覆盖率**：{X%}
- **构建结果**：{SUCCESS / FAILURE}
- **CI 触发时间**：{YYYY-MM-DD HH:mm}
- **CI 完成时间**：{YYYY-MM-DD HH:mm}

## 部署信息

- **目标环境**：{dev / test / staging / prod}
- **部署版本**：{vX.Y.Z}
- **部署方式**：{滚动更新 / 蓝绿部署 / 全量发布}
- **部署开始时间**：{YYYY-MM-DD HH:mm}
- **部署完成时间**：{YYYY-MM-DD HH:mm}
- **健康检查结果**：{通过 / 异常}

## 灰度发布信息

- **灰度比例**：{X%}
- **灰度策略**：{按用户 / 按地域 / 按流量}
- **灰度观察时长**：{X 小时}
- **灰度期间错误数**：{n}
- **灰度期间报警**：{无 / 有，已处理}

## 例外情况

- {记录任何偏离标准流程的情况及原因}
- {记录未解决的问题和后续跟进事项}

---

## 评审文件版本

| 文件 | 版本 | 日期 |
|------|------|------|
| spec.md | v1 | {YYYY-MM-DD} |
| review-v1.md | v1 | {YYYY-MM-DD} |
| review-v2.md | v2 | {YYYY-MM-DD} |
