# 量化交易系统

一个专业的量化交易软件系统，基于 **Rust + Tauri 2.0 + Vue3 + PostgreSQL + Redis** 技术栈构建。

## 📋 项目概述

本系统是一个完整的量化交易解决方案，涵盖数据管理、策略开发、回测分析、交易执行、风险管理和实时监控等核心功能。

### 核心特性

- ✅ **模块化架构**：高内聚低耦合的五大核心模块
- ✅ **多数据库支持**：PostgreSQL (关系型) + Redis (缓存) + InfluxDB (时序)
- ✅ **完整回测系统**：支持策略开发、参数优化、性能评估
- ✅ **智能执行算法**：TWAP、VWAP、冰山订单等
- ✅ **三层风控体系**：事前、事中、事后全流程风险管理
- ✅ **实时监控告警**：Prometheus 指标 + 多渠道告警
- ✅ **现代化界面**：Vue3 + Element Plus + ECharts

## 🏗️ 系统架构

```
quant-trading-system/
├── src-tauri/                 # Tauri 后端（Rust）
│   └── src/
│       ├── main.rs            # 主入口
│       ├── commands.rs        # Tauri 命令
│       └── state.rs           # 应用状态
├── crates/                    # Rust 模块
│   ├── common/                # 公共模块（类型、错误、工具）
│   ├── data-layer/            # 数据管理层
│   ├── strategy-layer/        # 策略开发层
│   ├── trading-layer/         # 交易执行层
│   ├── risk-layer/            # 风险管理层
│   └── monitor-layer/         # 监控告警层
├── src/                       # Vue3 前端
│   ├── views/                 # 视图页面
│   ├── router/                # 路由配置
│   └── main.ts                # 前端入口
├── Cargo.toml                 # Workspace 配置
├── package.json               # 前端依赖
└── .env.example               # 环境变量模板
```

## 🚀 快速开始

### 环境要求

- **Rust**: 1.77+
- **Node.js**: 18+
- **PostgreSQL**: 14+
- **Redis**: 6+
- **InfluxDB**: 2.x (可选)

### 安装步骤

1. **克隆项目**
```bash
git clone <repository-url>
cd ea_test
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑 .env 文件，配置数据库连接等信息
```

3. **安装前端依赖**
```bash
npm install
```

4. **运行开发环境**
```bash
# 方式1：同时启动前后端
npm run tauri dev

# 方式2：分别启动
npm run dev          # 启动前端
cargo tauri dev      # 启动 Tauri
```

5. **生产构建**
```bash
npm run build
npm run tauri build
```

## 📦 核心模块说明

### 1. 数据管理模块 (data-layer)

- **PostgreSQL**: 存储订单、持仓、账户等关系型数据
- **Redis**: 缓存热点数据，降低延迟
- **InfluxDB**: 存储高频行情时序数据
- **数据质量**：实时清洗、去重、异常检测

### 2. 策略开发模块 (strategy-layer)

- **策略接口**：统一的策略开发框架
- **技术指标**：SMA、EMA、RSI、MACD、布林带等
- **回测引擎**：高保真回测，包含滑点、手续费模拟
- **性能指标**：夏普比率、最大回撤、胜率等

### 3. 交易执行模块 (trading-layer)

- **订单管理**：订单生命周期管理
- **执行引擎**：支持模拟盘和实盘切换
- **算法交易**：TWAP、VWAP、冰山订单
- **延迟监控**：从信号到成交的全链路延迟统计

### 4. 风险管理模块 (risk-layer)

- **事前风控**：资金检查、持仓限制、集中度控制
- **实时监控**：账户风险、保证金、回撤监控
- **事后分析**：VaR 计算、压力测试、归因分析

### 5. 监控告警模块 (monitor-layer)

- **Prometheus 指标**：订单量、延迟、账户余额等
- **结构化日志**：基于 tracing 的分级日志系统
- **多渠道告警**：邮件、Webhook、企业微信

## 💡 使用示例

### 创建策略

```rust
use strategy_layer::{Strategy, StrategyContext};
use quant_common::types::{Order, StrategyParams};

struct MyStrategy {
    params: StrategyParams,
}

#[async_trait]
impl Strategy for MyStrategy {
    async fn generate_signals(&self, context: &StrategyContext) -> Result<Vec<Order>> {
        // 策略逻辑
        Ok(vec![])
    }
}
```

### 运行回测

```rust
use strategy_layer::BacktestEngine;

let mut engine = BacktestEngine::new(
    Decimal::new(1000000, 0),  // 初始资金
    Decimal::new(3, 4),         // 手续费 0.03%
    Decimal::new(1, 4),         // 滑点 0.01%
);

let result = engine.run(&strategy, market_data).await?;
println!("年化收益: {}", result.annual_return);
println!("夏普比率: {}", result.sharpe_ratio);
println!("最大回撤: {}", result.max_drawdown);
```

## 📊 数据库迁移

```bash
# 创建数据库
createdb quant_trading

# 运行迁移（需要实现）
# sqlx migrate run
```

## 🔒 安全与合规

- ✅ 数据传输加密（TLS）
- ✅ API 密钥加密存储
- ✅ 操作审计日志
- ✅ IP 白名单
- ✅ 双因子认证（2FA）

## 📈 性能优化

- 向量化计算（Rust + ndarray）
- Redis 缓存热点数据
- 异步 IO（Tokio）
- 连接池管理
- 批量操作优化

## 🛣️ 开发路线图

### 第一阶段（已完成）✅
- [x] 项目架构搭建
- [x] 数据层实现
- [x] 策略层实现
- [x] 交易层实现
- [x] 风控层实现
- [x] 监控层实现
- [x] 前端界面框架

### 第二阶段（进行中）
- [ ] 数据库迁移脚本
- [ ] 交易所 API 对接
- [ ] 完善回测功能
- [ ] WebSocket 实时行情
- [ ] 更多技术指标

### 第三阶段（计划中）
- [ ] 机器学习模型集成
- [ ] 多市场支持
- [ ] 高频交易优化
- [ ] 分布式部署
- [ ] 策略市场

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT License

## ⚠️ 免责声明

本软件仅供学习和研究使用。量化交易存在风险，请谨慎使用真实资金。过往业绩不代表未来收益。
