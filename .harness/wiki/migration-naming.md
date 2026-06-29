# Migration 命名规范

> SQL migration 文件的命名、编号、提交流程的统一规则。
> CI 通过 `.harness/scripts/verify-migrations.sh` 自动校验。

## 文件命名格式

```
YYYYMMDDxxxxxx_subject.sql
│       │       │
│       │       └─ 主题（下划线连接的小写蛇形）
│       └─ 6 位顺序号（000001 开始，连续递增）
└─ 固定为 20240101（项目启动日期，不可变）
```

**正则**：`^2024010100[0-9]{4}_[a-z][a-z0-9_]*\.sql$`

### 主题命名

- 小写字母开头
- 只允许 `[a-z0-9_]`
- 描述本 migration 的内容（用户能从一个名字看出版本作用）

✅ 推荐：
- `20240101000001_create_initial_tables.sql`
- `20240101000013_add_strategy_status_and_fields.sql`
- `20240101000016_align_schemas_with_code.sql`

❌ 禁止：
- `20260629000001_add_strategy_version.sql` — 日期前缀必须是 `20240101`
- `Migration_001.sql` — 大写、无顺序号
- `add-column.sql` — 缺顺序号

## 编号规则

- 从 `000001` 开始
- 顺序递增，无跳号
- 顺序号反映**应用顺序**，不是创建时间
- 已删除的 migration **永远不要**重排号（保持稳定）

### 保留编号

| 编号 | 状态 | 原因 |
|------|------|------|
| `000012` | **保留** | 故意保留作未来插入位置 |

保留位的文件**禁止**创建（CI 报错）。如果团队决定未来真的需要在该位插入，应先在 Wiki 注明，再补一条 migration。

## 编写规则

### 必须做到

1. **幂等性**：所有 DDL 使用 `IF NOT EXISTS` / `DO $$ ... $$` 块，可重复执行
2. **向后兼容**：`ADD COLUMN` 必须有 `DEFAULT`，否则旧数据为 NULL 会破坏现有查询
3. **可逆性**：每个文件底部留 `-- Down migration` 注释示例（即使不实现 `undo()`）
4. **命名说明**：每个文件第 1 行写 `Migration NNN: <一句话描述>`
5. **架构边界**：分区的 `UNIQUE` 约束必须包含分区键

### 禁止

1. **不要**在 `IF NOT EXISTS` 失败时直接 panic —— 写在 `DO $$ ... EXCEPTION WHEN ... END $$`
2. **不要**修改已应用的 migration 文件 —— 创建新 migration 修正
3. **不要**在生产库应用前删除已应用的 migration 文件
4. **不要**在 migration 中写 Rust 代码或外部命令 —— 纯 SQL
5. **不要**省略 `IF NOT EXISTS` —— 每次重跑都需成功

## 流程

```
1. 创建 migration:    cp <template> crates/data-layer/migrations/20240101000NNN_subject.sql
2. 本地验证:          bash .harness/scripts/verify-migrations.sh
3. 集成测试:          cargo test -p data-layer --test migration_integration -- --ignored
4. 提交:              git commit -m "feat(migration): NNN subject"
5. CI 自动校验:       .github/workflows/harness-ci.yml 跑 verify-migrations.sh
```

## 加载机制

- **编译时嵌入**：`sqlx::migrate!("./migrations")` 宏在 `data-layer/src/postgres.rs:38`
- **运行时执行**：`PostgresClient::run_migrations()` 在 `src-tauri/src/main.rs:119` 启动时调用
- **手动工具**：`src-tauri/src/bin/migrate.rs` 提供独立 CLI

## 历史命名违规

| 文件 | 违规类型 | 已修复？ |
|------|---------|---------|
| `20260629000001_add_strategy_version.sql` | 日期前缀错误 | ✅ 重命名为 `20240101000015` |

## 校验脚本退出码

| 退出码 | 含义 | CI 行为 |
|--------|------|---------|
| 0 | 全部检查通过 | 继续 |
| 1 | 至少一项检查失败 | 阻塞 PR |
