# Unit Test Write Skill

> 单元测试编写技能。遵循"改动驱动测试"原则：改了哪个接口就测哪个接口。

## 输入
- 变更涉及的服务接口/方法列表
- 线上真实请求出入参（通过 MCP 工具获取，优先）

## 输出
- 新增/修改的单元测试文件

## 核心原则

### 改动驱动测试（Change-driven Testing）
- 改了哪个接口就测哪个接口，而非一刀切测最上层
- 新增方法必须新增对应的测试方法
- 修改方法需要补充新的测试 case 覆盖修改逻辑

### 数据来源优先级
1. 线上真实请求出入参（通过 MCP 工具查询）— 最高优先级
2. 线下测试环境的请求记录
3. 根据业务逻辑手动构造

## 执行步骤

### Step 1：确定测试范围
1. 分析变更影响到的接口和方法
2. 对每个受影响的接口，确定测试场景
3. 记录测试范围到 tasks.md

### Step 2：获取测试数据
1. 优先通过 MCP 工具查询线上真实请求日志
2. 获取请求入参和响应出参
3. 对敏感数据进行脱敏处理
4. 如果没有线上数据，手动构造符合业务逻辑的测试数据

### Step 3：编写测试代码
按照 Arrange-Act-Assert 三段式结构编写：

```rust
#[tokio::test]
async fn test_get_price_item_exists_returns_price() {
    // Arrange
    let item_id = ItemId(12345);
    let expected_price = Money::from_cents(9990);
    let mut mock_repo = MockPriceRuleRepository::new();
    mock_repo.expect_find_active_by_item_id()
        .return_once(move |_, _| Ok(vec![build_price_row(item_id, expected_price)]));
    let service = DefaultPriceService::new(Arc::new(mock_repo), Arc::new(MockPriceCache::new()));

    // Act
    let result = service.get_price(item_id).await.unwrap();

    // Assert
    assert_eq!(result, Some(expected_price));
}

#[tokio::test]
async fn test_get_price_item_not_exists_returns_none() {
    // Arrange
    let item_id = ItemId(99999);
    let mut mock_repo = MockPriceRuleRepository::new();
    mock_repo.expect_find_active_by_item_id()
        .return_once(|_, _| Ok(vec![]));
    let service = DefaultPriceService::new(Arc::new(mock_repo), Arc::new(MockPriceCache::new()));

    // Act
    let result = service.get_price(item_id).await.unwrap();

    // Assert
    assert_eq!(result, None);
}
```

### Step 4：运行测试
1. 本地运行所有测试
2. 确认全部通过（passed == total）
3. 确认新增测试无 flaky

## 测试规范

### 测试框架
- 使用 `#[cfg(test)] mod tests` + `#[tokio::test]`
- Mock 通过 trait object 实现（`MockXxxRepository` 实现对应 trait）
- 优先使用 `mockall` crate 自动生成 mock
- 集成测试使用 `sqlx::test` 或 testcontainers

### 命名规范
- 类名：`{被测试类名}Test`
- 方法名：`test_{function_name}_{scenario}_{expected_result}`

### 覆盖率要求
- 新增代码行覆盖率 ≥ 80%
- 新增代码分支覆盖率 ≥ 70%
- 核心逻辑（价格计算、状态判断）需要 100% 分支覆盖

### 禁止的行为
- 禁止测试依赖外部环境（DB、缓存、RPC 需 Mock）
- 禁止测试之间有顺序依赖
- 禁止在测试中 sleep
- 禁止测试抛出未捕获异常
