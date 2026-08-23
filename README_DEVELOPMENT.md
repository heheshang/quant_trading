# 量化交易系统开发文档

## 文档目录

1. [开发指南](DEVELOPMENT_GUIDE.md) - 系统架构、环境配置、开发规范
2. [API 文档](API_DOCUMENTATION.md) - Tauri 命令接口详细说明
3. [开发流程](DEVELOPMENT_WORKFLOW.md) - 开发、测试、部署完整流程

## 快速开始

### 环境要求

- **Rust**: 1.77+
- **Node.js**: 18+
- **PostgreSQL**: 14+
- **Redis**: 6+
- **历史行情**: PostgreSQL 分区表（不使用 InfluxDB）

### 安装步骤

```bash
# 1. 克隆项目
git clone <repository-url>
cd quant-trading-system

# 2. 配置环境变量
cp .env.example .env
# 编辑 .env 文件配置数据库等信息

# 3. 安装依赖
npm install

# 4. 启动开发环境
npm run tauri dev
```

## 项目结构

```
quant-trading-system/
├── src-tauri/              # Tauri 后端（Rust）
├── crates/                 # Rust 模块
│   ├── common/             # 公共模块
│   ├── data-layer/         # 数据管理层
│   ├── strategy-layer/     # 策略开发层
│   ├── trading-layer/      # 交易执行层
│   ├── risk-layer/         # 风险管理层
│   └── monitor-layer/      # 监控告警层
├── src/                    # Vue3 前端
├── docs/                   # 详细文档
├── DEVELOPMENT_GUIDE.md    # 开发指南
├── API_DOCUMENTATION.md    # API 文档
├── DEVELOPMENT_WORKFLOW.md # 开发流程
└── README_DEVELOPMENT.md   # 开发文档说明
```

## 核心功能模块

### 1. 数据管理层
- PostgreSQL 关系型数据存储
- Redis 缓存热点数据
- 历史行情存 PostgreSQL 分区表（不使用 InfluxDB）
- 数据质量检查机制

### 2. 策略开发层
- 统一策略开发框架
- 内置技术指标（SMA、EMA、RSI、MACD、布林带等）
- 回测引擎（含滑点和手续费模拟）
- 性能指标计算（夏普比率、最大回撤、胜率等）

### 3. 交易执行层
- 订单生命周期管理
- 执行引擎（支持纸面交易和实盘）
- 算法订单（TWAP、VWAP、冰山订单）
- 全链路延迟监控

### 4. 风险管理层
- 事前风控（资金、持仓限制）
- 实时监控（保证金、回撤）
- 事后分析（VaR、压力测试、归因分析）

### 5. 监控告警层
- Prometheus 指标收集
- 结构化日志系统
- 多渠道告警（邮件、Webhook、企业微信）

## 技术栈

- **后端**: Rust + Tauri 2.0
- **前端**: Vue 3 + TypeScript + Element Plus + ECharts
- **数据库**: PostgreSQL (含历史行情分区表) + Redis
- **构建工具**: Cargo + Vite + vue-tsc
- **异步运行时**: Tokio
- **序列化**: serde + bincode + JSON

## 开发规范概览

### 代码规范
- Rust: 使用 rustfmt 格式化代码
- TypeScript: 使用 Prettier 格式化代码
- 命名规范: 模块使用 kebab-case，结构体使用 PascalCase，函数使用 snake_case

### 测试规范
- 单元测试覆盖率要求 80% 以上
- 集成测试覆盖核心业务流程
- 所有功能需有对应测试用例

### 安全规范
- 敏感数据加密存储
- JWT Token 身份验证
- API 请求签名验证
- 定期安全审计

## 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 技术支持

如有问题，请提交 Issue 或联系开发团队。