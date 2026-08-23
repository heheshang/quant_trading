# Docker 部署与验证

## 用途

仓库默认面向 Tauri 桌面应用运行。本目录提供一套可重复的容器部署方案，用于验证：

- PostgreSQL / Redis 基础设施健康检查。
- SQLx 数据库迁移在容器内执行。
- 前端生产构建和 Tauri Linux release 构建。
- Tauri 应用在 Xvfb 无头环境下启动、连接数据库和 Redis。

## 快速启动

```bash
# 构建镜像并启动全部服务
docker compose build app
docker compose up -d

# 查看状态和日志
docker compose ps
docker compose logs -f app
```

默认宿主端口为：

- PostgreSQL：`127.0.0.1:15432`
- Redis：`127.0.0.1:16379`

可以通过环境变量覆盖，例如：

```bash
POSTGRES_PORT=5433 REDIS_PORT=6380 docker compose up -d
```

## 一键部署测试

```bash
bash scripts/docker-test.sh
```

该脚本依次执行：

1. 构建 `quant-trading-system:docker-test` 镜像。
2. 启动 PostgreSQL 和 Redis 并等待健康。
3. 运行 `migrate-db up`。
4. 查询 `information_schema.tables` 确认迁移已落库。
5. 运行一次容器内应用冒烟测试。
6. 启动常驻 `app` 服务并等待健康检查通过。

## 已验证结果

2026-08-02 在 macOS + Colima（Linux arm64）上完成全流程验证：

- `quant-trading-system:docker-test` 镜像构建成功，包含前端生产构建和 Tauri Linux release。
- PostgreSQL / Redis 健康检查通过。
- `migrate-db up` 在全新数据库上执行成功，`public` schema 共 54 张表。
- 应用容器成功连接 PostgreSQL 和 Redis，日志输出 `Application initialized successfully`。
- 常驻 `app` 服务健康检查通过，随后测试资源已通过 `docker compose down -v` 清理。

## 构建资源要求

Tauri Linux release 编译内存占用较高。Docker VM 至少需要 4 CPU / 8GB 内存；
Colima 可这样启动：

```bash
colima start --cpu 4 --memory 8
```

## 关键环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DATABASE_HOST` | `postgres` | PostgreSQL 服务名 |
| `DATABASE_PORT` | `5432` | PostgreSQL 容器端口 |
| `DATABASE_USERNAME` | `quant` | 数据库用户 |
| `DATABASE_PASSWORD` | `quant_password` | 数据库密码 |
| `DATABASE_NAME` | `quant_trading` | 数据库名 |
| `REDIS_HOST` | `redis` | Redis 服务名 |
| `REDIS_PASSWORD` | 空 | Redis 密码 |
| `BINANCE_ENABLE` | `false` | 默认关闭外部交易所依赖，便于离线测试 |
| `JWT_SECRET` | `docker_test_change_me` | 生产环境必须覆盖 |

## 手动验证

```bash
# 只运行迁移
docker compose run --rm migrate up

# 单次应用冒烟
docker compose run --rm app smoke

# 进入运行中的 app 容器
docker compose exec app sh

# 清理全部容器和测试数据卷
docker compose down -v
```
