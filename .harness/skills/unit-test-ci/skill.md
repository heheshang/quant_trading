# Unit Test CI Skill

> CI 流水线测试技能。用于单元测试 CI 阶段，管理代码提交和 CI 验证。

## 输入
- 变更后的源码和测试文件
- CI 配置信息

## 输出
- CI 执行结果报告

## 执行步骤

### Step 1：提交前检查
- [ ] 编译通过（本地）
- [ ] 全部测试通过（本地）
- [ ] 无未提交的临时文件
- [ ] 变更文件清单与 tasks.md 保持一致

### Step 2：提交代码
```
git add {变更文件}
git commit -m "{type}: {description}

{详细描述变更内容}

- {文件}: {变更说明}
- {文件}: {变更说明}
"
```

### Step 3：触发 CI
- 确保 CI pipeline 已触发
- 记录 CI 触发时间和编号

### Step 4：监控 CI 执行
- 等待 CI pipeline 完成
- 收集执行日志

### Step 5：验证 CI 结果

验证条件（需要全部满足）：
```
status == SUCCESS
total_tests > 0
passed == total_tests
```

### Step 6：处理 CI 失败

| 失败类型 | 处理方式 | 回退到 |
|----------|----------|--------|
| 编译错误 | 检查编译错误信息，修复代码 | 阶段 3（编码实现） |
| 测试失败 | 检查失败用例，修复测试或代码 | 阶段 5（测试编写） |
| 测试为 0/0 | 检查测试是否被正确编写和执行 | 阶段 5（测试编写） |
| Lint 检测失败 | 修复代码规范问题 | 阶段 3（编码实现） |

## CI 配置参考

```yaml
# .harness/ci/config.yml
# 构建命令由 detect-build.sh 自动适配
pipeline:
  stages:
    - compile:
        script: source .harness/scripts/detect-build.sh && ${BUILD_CMD}
    - test:
        script: source .harness/scripts/detect-build.sh && ${TEST_CMD}
        reports:
          - target/nextest/ci/junit.xml           # Rust (cargo nextest)
        coverage:
          threshold: 80
    - lint:
        script: source .harness/scripts/detect-build.sh && ${LINT_CMD}
```
